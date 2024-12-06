use std::io::Write;
use std::os::fd::IntoRawFd;
use std::ptr::{null, null_mut};
use std::thread::sleep;

use vglite_rs::{Context, Buffer, Color, Format, Rectangle};

use drm::Device;
use drm::control::Device as ControlDevice;
use drm::buffer::{Buffer as DrmBuffer, DrmFourcc};
use drm::control::{self, atomic, connector, crtc, property, AtomicCommitFlags};

use std::fs::File;
use std::fs::OpenOptions;

use std::os::unix::io::AsFd;
use std::os::unix::io::BorrowedFd;

#[derive(Debug)]
struct Card(File);

/// Implementing [`AsFd`] is a prerequisite to implementing the traits found
/// in this crate. Here, we are just calling [`File::as_fd()`] on the inner
/// [`File`].
impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

/// With [`AsFd`] implemented, we can now implement [`drm::Device`].
impl Device for Card {}
impl ControlDevice for Card {}

impl Card {
    /// Simple helper method for opening a [`Card`].
    fn open() -> Self {
        let mut options = OpenOptions::new();
        options.read(true);
        options.write(true);

        // The normal location of the primary device node on Linux
        let mut card = Card(options.open("/dev/dri/card0").unwrap());
        // card.set_client_capability(drm::ClientCapability::UniversalPlanes, true)
        //     .expect("Unable to request UniversalPlanes capability");
        // card.set_client_capability(drm::ClientCapability::Atomic, true)
        //     .expect("Unable to request Atomic capability");

        // // Load the information.
        // let res = card
        //     .resource_handles()
        //     .expect("Could not load normal resource ids.");
        // let conninfo: Vec<connector::Info> = res
        //     .connectors()
        //     .iter()
        //     .flat_map(|con| card.get_connector(*con, true))
        //     .collect();
        // let crtcinfo: Vec<crtc::Info> = res
        //     .crtcs()
        //     .iter()
        //     .flat_map(|crtc| card.get_crtc(*crtc))
        //     .collect();

        // // Filter each connector until we find one that's connected.
        // let con = conninfo
        //     .iter()
        //     .find(|&i| i.state() == connector::State::Connected)
        //     .expect("No connected connectors");

        card
    }

    fn choose_mode(&mut self) {}
}

fn main() {
    let card = Card::open();
    card.set_client_capability(drm::ClientCapability::UniversalPlanes, true)
        .expect("Unable to request UniversalPlanes capability");
    card.set_client_capability(drm::ClientCapability::Atomic, true)
        .expect("Unable to request Atomic capability");

    // Load the information.
    let res = card
        .resource_handles()
        .expect("Could not load normal resource ids.");
    let coninfo: Vec<connector::Info> = res
        .connectors()
        .iter()
        .flat_map(|con| card.get_connector(*con, true))
        .collect();
    let crtcinfo: Vec<crtc::Info> = res
        .crtcs()
        .iter()
        .flat_map(|crtc| card.get_crtc(*crtc))
        .collect();

    // Filter each connector until we find one that's connected.
    let con = coninfo
        .iter()
        .find(|&i| i.state() == connector::State::Connected)
        .expect("No connected connectors");

    let &mode = con.modes().first().expect("No modes found on connector");
    let (disp_width, disp_height) = mode.size();
    // Find a crtc and FB
    let crtc = crtcinfo.first().expect("No crtcs found");
    // Select the pixel format
    let fmt = DrmFourcc::Argb8888;
    // Create a DB
    // If buffer resolution is above display resolution, a ENOSPC (not enough GPU memory) error may
    // occur
    let mut db = card
        .create_dumb_buffer((disp_width.into(), disp_height.into()), fmt, 32)
        .expect("Could not create dumb buffer");

    // Create an FB:
    let fb = card
        .add_framebuffer(&db, 32, 32)
        .expect("Could not create FB");
    let planes = card.plane_handles().expect("Could not list planes");
    let (better_planes, compatible_planes): (
        Vec<control::plane::Handle>,
        Vec<control::plane::Handle>,
    ) = planes
        .iter()
        .filter(|&&plane| {
            card.get_plane(plane)
                .map(|plane_info| {
                    let compatible_crtcs = res.filter_crtcs(plane_info.possible_crtcs());
                    compatible_crtcs.contains(&crtc.handle())
                })
                .unwrap_or(false)
        })
        .partition(|&&plane| {
            if let Ok(props) = card.get_properties(plane) {
                for (&id, &val) in props.iter() {
                    if let Ok(info) = card.get_property(id) {
                        if info.name().to_str().map(|x| x == "type").unwrap_or(false) {
                            return val == (drm::control::PlaneType::Primary as u32).into();
                        }
                    }
                }
            }
            false
        });
    let fd = card.buffer_to_prime_fd(db.handle(), 0).expect("Could not get prime fd");
    let plane = *better_planes.first().unwrap_or(&compatible_planes[0]);
    let con_props = card
        .get_properties(con.handle())
        .expect("Could not get props of connector")
        .as_hashmap(&card)
        .expect("Could not get a prop from connector");
    let crtc_props = card
        .get_properties(crtc.handle())
        .expect("Could not get props of crtc")
        .as_hashmap(&card)
        .expect("Could not get a prop from crtc");
    let plane_props = card
        .get_properties(plane)
        .expect("Could not get props of plane")
        .as_hashmap(&card)
        .expect("Could not get a prop from plane");
    let mut atomic_req = atomic::AtomicModeReq::new();
    atomic_req.add_property(
        con.handle(),
        con_props["CRTC_ID"].handle(),
        property::Value::CRTC(Some(crtc.handle())),
    );
    let blob = card
        .create_property_blob(&mode)
        .expect("Failed to create blob");
    atomic_req.add_property(crtc.handle(), crtc_props["MODE_ID"].handle(), blob);
    atomic_req.add_property(
        crtc.handle(),
        crtc_props["ACTIVE"].handle(),
        property::Value::Boolean(true),
    );
    atomic_req.add_property(
        plane,
        plane_props["FB_ID"].handle(),
        property::Value::Framebuffer(Some(fb)),
    );
    atomic_req.add_property(
        plane,
        plane_props["CRTC_ID"].handle(),
        property::Value::CRTC(Some(crtc.handle())),
    );
    atomic_req.add_property(
        plane,
        plane_props["SRC_X"].handle(),
        property::Value::UnsignedRange(0),
    );
    atomic_req.add_property(
        plane,
        plane_props["SRC_Y"].handle(),
        property::Value::UnsignedRange(0),
    );
    atomic_req.add_property(
        plane,
        plane_props["SRC_W"].handle(),
        property::Value::UnsignedRange((mode.size().0 as u64) << 16),
    );
    atomic_req.add_property(
        plane,
        plane_props["SRC_H"].handle(),
        property::Value::UnsignedRange((mode.size().1 as u64) << 16),
    );
    atomic_req.add_property(
        plane,
        plane_props["CRTC_X"].handle(),
        property::Value::SignedRange(0),
    );
    atomic_req.add_property(
        plane,
        plane_props["CRTC_Y"].handle(),
        property::Value::SignedRange(0),
    );
    atomic_req.add_property(
        plane,
        plane_props["CRTC_W"].handle(),
        property::Value::UnsignedRange(mode.size().0 as u64),
    );
    atomic_req.add_property(
        plane,
        plane_props["CRTC_H"].handle(),
        property::Value::UnsignedRange(mode.size().1 as u64),
    );

    let buffer_width = db.size().0;
    let buffer_height = db.size().1;

    // Set the crtc
    // On many setups, this requires root access.
    card.atomic_commit(AtomicCommitFlags::ALLOW_MODESET, atomic_req)
        .expect("Failed to set mode");

    let ctx = Context::new(buffer_width, buffer_height).unwrap();
    let mut map = card
            .map_dumb_buffer(&mut db)
            .expect("Could not map dumbbuffer");
    let mut buffer = Buffer::map(buffer_width, buffer_height, Format::RGBA8888, fd.into_raw_fd(), map.as_mut_ptr() as _)
       .expect("Could not map buffer");
    // let mut buffer = Buffer::allocate(640, 480, Format::RGBA8888).unwrap();
    for i in 0..1000 {
        buffer.clear(None, Color { r: 0, g: 0, b: 0, a: 255 }).unwrap();
        buffer.clear(Some(&mut Rectangle { x: 300, y: 370, width: 300 + i, height: 100 }), Color { r: 255, g: 0, b: 0, a: 255 }).unwrap();
        ctx.finish().unwrap();
        sleep(std::time::Duration::from_millis(16));
    }
}
