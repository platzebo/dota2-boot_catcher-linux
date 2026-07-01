#![allow(dead_code)]

use std::env;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::os::raw::{c_char, c_int, c_long, c_uchar, c_uint, c_ulong, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

static STOP: AtomicBool = AtomicBool::new(false);

type Bool = c_int;
type Window = c_ulong;
type Atom = c_ulong;
type KeySym = c_ulong;

type Display = c_void;
type Visual = c_void;
type Screen = c_void;

type Colormap = c_ulong;

const XK_A: KeySym = 0x0041;
const XK_D: KeySym = 0x0044;
const XK_SPACE: KeySym = 0x0020;
const ZPIXMAP: c_int = 2;
const ANY_PROPERTY_TYPE: Atom = 0;
const SUCCESS: c_int = 0;
const IS_VIEWABLE: c_int = 2;
const LSB_FIRST: c_int = 0;
const CURRENT_TIME: c_ulong = 0;
const REVERT_TO_PARENT: c_int = 2;
const IPC_PRIVATE: c_int = 0;
const IPC_CREAT: c_int = 0o1000;
const IPC_RMID: c_int = 0;

// Keep the old names used by the original control code.
const VK_A: KeySym = XK_A;
const VK_D: KeySym = XK_D;
const VK_SPACE: KeySym = XK_SPACE;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct XWindowAttributes {
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border_width: c_int,
    depth: c_int,
    visual: *mut Visual,
    root: Window,
    class: c_int,
    bit_gravity: c_int,
    win_gravity: c_int,
    backing_store: c_int,
    backing_planes: c_ulong,
    backing_pixel: c_ulong,
    save_under: Bool,
    colormap: Colormap,
    map_installed: Bool,
    map_state: c_int,
    all_event_masks: c_long,
    your_event_mask: c_long,
    do_not_propagate_mask: c_long,
    override_redirect: Bool,
    screen: *mut Screen,
}

#[repr(C)]
struct XImage {
    width: c_int,
    height: c_int,
    xoffset: c_int,
    format: c_int,
    data: *mut c_char,
    byte_order: c_int,
    bitmap_unit: c_int,
    bitmap_bit_order: c_int,
    bitmap_pad: c_int,
    depth: c_int,
    bytes_per_line: c_int,
    bits_per_pixel: c_int,
    red_mask: c_ulong,
    green_mask: c_ulong,
    blue_mask: c_ulong,
}

#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
    fn XCloseDisplay(display: *mut Display) -> c_int;
    fn XDefaultRootWindow(display: *mut Display) -> Window;
    fn XDefaultScreen(display: *mut Display) -> c_int;
    fn XQueryTree(
        display: *mut Display,
        w: Window,
        root_return: *mut Window,
        parent_return: *mut Window,
        children_return: *mut *mut Window,
        nchildren_return: *mut c_uint,
    ) -> c_int;
    fn XFetchName(display: *mut Display, w: Window, window_name_return: *mut *mut c_char) -> c_int;
    fn XFree(data: *mut c_void) -> c_int;
    fn XInternAtom(display: *mut Display, atom_name: *const c_char, only_if_exists: Bool) -> Atom;
    fn XGetWindowProperty(
        display: *mut Display,
        w: Window,
        property: Atom,
        long_offset: c_long,
        long_length: c_long,
        delete: Bool,
        req_type: Atom,
        actual_type_return: *mut Atom,
        actual_format_return: *mut c_int,
        nitems_return: *mut c_ulong,
        bytes_after_return: *mut c_ulong,
        prop_return: *mut *mut c_uchar,
    ) -> c_int;
    fn XGetWindowAttributes(
        display: *mut Display,
        w: Window,
        window_attributes_return: *mut XWindowAttributes,
    ) -> c_int;
    fn XTranslateCoordinates(
        display: *mut Display,
        src_w: Window,
        dest_w: Window,
        src_x: c_int,
        src_y: c_int,
        dest_x_return: *mut c_int,
        dest_y_return: *mut c_int,
        child_return: *mut Window,
    ) -> Bool;
    fn XGetImage(
        display: *mut Display,
        d: Window,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
        plane_mask: c_ulong,
        format: c_int,
    ) -> *mut XImage;
    fn XDestroyImage(ximage: *mut XImage) -> c_int;
    fn XKeysymToKeycode(display: *mut Display, keysym: KeySym) -> c_uint;
    fn XRaiseWindow(display: *mut Display, w: Window) -> c_int;
    fn XSetInputFocus(
        display: *mut Display,
        focus: Window,
        revert_to: c_int,
        time: c_ulong,
    ) -> c_int;
    fn XSetErrorHandler(
        handler: Option<unsafe extern "C" fn(*mut Display, *mut XErrorEvent) -> c_int>,
    ) -> Option<unsafe extern "C" fn(*mut Display, *mut XErrorEvent) -> c_int>;
    fn XSync(display: *mut Display, discard: Bool) -> c_int;
    fn XFlush(display: *mut Display) -> c_int;
}

#[repr(C)]
struct XErrorEvent {
    type_: c_int,
    display: *mut Display,
    resourceid: c_ulong,
    serial: c_ulong,
    error_code: c_uchar,
    request_code: c_uchar,
    minor_code: c_uchar,
}

static X_ERROR_SEEN: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn x_error_handler(_display: *mut Display, _event: *mut XErrorEvent) -> c_int {
    X_ERROR_SEEN.store(true, Ordering::SeqCst);
    0
}

#[link(name = "Xtst")]
unsafe extern "C" {
    fn XTestFakeKeyEvent(
        display: *mut Display,
        keycode: c_uint,
        is_press: Bool,
        delay: c_ulong,
    ) -> c_int;
    fn XTestFakeButtonEvent(
        display: *mut Display,
        button: c_uint,
        is_press: Bool,
        delay: c_ulong,
    ) -> c_int;
    fn XTestFakeMotionEvent(
        display: *mut Display,
        screen_number: c_int,
        x: c_int,
        y: c_int,
        delay: c_ulong,
    ) -> c_int;
}

#[repr(C)]
struct XShmSegmentInfo {
    shmseg: c_ulong,
    shmid: c_int,
    shmaddr: *mut c_char,
    read_only: Bool,
}

#[link(name = "Xext")]
unsafe extern "C" {
    fn XShmQueryExtension(display: *mut Display) -> Bool;
    fn XShmCreateImage(
        display: *mut Display,
        visual: *mut Visual,
        depth: c_uint,
        format: c_int,
        data: *mut c_char,
        shminfo: *mut XShmSegmentInfo,
        width: c_uint,
        height: c_uint,
    ) -> *mut XImage;
    fn XShmAttach(display: *mut Display, shminfo: *mut XShmSegmentInfo) -> Bool;
    fn XShmDetach(display: *mut Display, shminfo: *mut XShmSegmentInfo) -> Bool;
    fn XShmGetImage(
        display: *mut Display,
        d: Window,
        image: *mut XImage,
        x: c_int,
        y: c_int,
        plane_mask: c_ulong,
    ) -> Bool;
}

unsafe extern "C" {
    fn shmget(key: c_int, size: usize, shmflg: c_int) -> c_int;
    fn shmat(shmid: c_int, shmaddr: *const c_void, shmflg: c_int) -> *mut c_void;
    fn shmdt(shmaddr: *const c_void) -> c_int;
    fn shmctl(shmid: c_int, cmd: c_int, buf: *mut c_void) -> c_int;
}

unsafe extern "C" {
    fn signal(signum: c_int, handler: unsafe extern "C" fn(c_int)) -> usize;
}

unsafe extern "C" fn signal_handler(_sig: c_int) {
    STOP.store(true, Ordering::SeqCst);
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

#[derive(Clone, Copy, Debug)]
struct Roi {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Clone, Copy, Debug)]
struct BootSize {
    w: i32,
    h: i32,
}

#[derive(Clone, Copy, Debug)]
struct Detection {
    x: i32,
    y: i32,
    score: f32,
    bbox: Roi,
    source: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct MotionState {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    last_t: f64,
    lost_frames: i32,
    seen_frames: i32,
    score: f32,
}

impl MotionState {
    fn speed(self) -> f32 {
        (self.vx * self.vx + self.vy * self.vy).sqrt()
    }
}

#[derive(Clone, Debug)]
struct Args {
    fps: f64,
    log_file: String,
    template: String,
    window_title: String,
    search_top: f32,
    search_bottom: f32,
    search_margin: f32,
    cart_y: f32,
    cart_speed: f32,
    deadzone: f32,
    min_boot_speed: f32,
    lost_keep_frames: i32,
    roi_padding: i32,
    roi_speed: f32,
    lost_roi_grow: i32,
    boot_size_max_scale: f32,
    intercept_horizon: f32,
    physics_step: f32,
    boot_radius: i32,
    debug: bool,
    no_startup: bool,
    click_focus: bool,
    list_windows: bool,
    control: bool,
    dry_run_frames: Option<u64>,
    use_shm: bool,
    full_window: bool,
    game_rect: Option<Rect>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            fps: 60.0,
            log_file: "boot_catcher_debug.tsv".to_string(),
            template: "boot_flying_template.png".to_string(),
            window_title: "Dota 2".to_string(),
            search_top: 0.15,
            search_bottom: 0.96,
            search_margin: 0.04,
            cart_y: 0.89,
            cart_speed: 430.0,
            deadzone: 18.0,
            min_boot_speed: 180.0,
            lost_keep_frames: 20,
            roi_padding: 28,
            roi_speed: 0.045,
            lost_roi_grow: 14,
            boot_size_max_scale: 1.45,
            intercept_horizon: 2.2,
            physics_step: 0.018,
            boot_radius: 16,
            debug: false,
            no_startup: false,
            click_focus: true,
            list_windows: false,
            control: true,
            dry_run_frames: None,
            use_shm: false,
            full_window: false,
            game_rect: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HeldKey {
    None,
    A,
    D,
}

fn parse_rect_spec(spec: &str) -> Option<Rect> {
    let parts: Vec<i32> = spec
        .split(',')
        .map(|s| s.trim().parse::<i32>())
        .collect::<Result<_, _>>()
        .ok()?;
    if parts.len() != 4 || parts[2] <= 0 || parts[3] <= 0 {
        return None;
    }
    Some(Rect {
        left: parts[0],
        top: parts[1],
        width: parts[2],
        height: parts[3],
    })
}

fn default_dota_boot_catcher_rect(window_width: i32, window_height: i32) -> Rect {
    // The Dark Carnival Boot Catcher playfield is a centered modal within the Dota window.
    // These ratios crop to the actual black playfield + cart lane, excluding the side UI/art
    // that previously caused false boot/cart detections and constant right steering.
    Rect {
        left: (window_width as f32 * 0.333).round() as i32,
        top: (window_height as f32 * 0.143).round() as i32,
        width: (window_width as f32 * 0.336).round() as i32,
        height: (window_height as f32 * 0.795).round() as i32,
    }
}

fn clamp_rect_to_window(mut r: Rect, window_width: i32, window_height: i32) -> Rect {
    r.left = r.left.clamp(0, window_width.saturating_sub(1));
    r.top = r.top.clamp(0, window_height.saturating_sub(1));
    r.width = r.width.clamp(1, window_width - r.left);
    r.height = r.height.clamp(1, window_height - r.top);
    r
}

fn parse_args() -> Args {
    let mut args = Args::default();
    let mut it = env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--fps" => args.fps = it.next().and_then(|v| v.parse().ok()).unwrap_or(args.fps),
            "--log-file" => args.log_file = it.next().unwrap_or_default(),
            "--template" => args.template = it.next().unwrap_or(args.template),
            "--window-title" | "--title" => {
                args.window_title = it.next().unwrap_or(args.window_title)
            }
            "--debug" => args.debug = true,
            "--no-startup" => args.no_startup = true,
            "--no-click-focus" => args.click_focus = false,
            "--no-control" => args.control = false,
            "--use-shm" => args.use_shm = true,
            "--full-window" => args.full_window = true,
            "--game-rect" => args.game_rect = it.next().and_then(|v| parse_rect_spec(&v)),
            "--dry-run-frames" => args.dry_run_frames = it.next().and_then(|v| v.parse().ok()),
            "--list-windows" => args.list_windows = true,
            "--deadzone" => {
                args.deadzone = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(args.deadzone)
            }
            "--cart-speed" => {
                args.cart_speed = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(args.cart_speed)
            }
            "--min-boot-speed" => {
                args.min_boot_speed = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(args.min_boot_speed)
            }
            "--help" | "-h" => {
                println!(
                    "Usage: boot_catcher_rs [--fps N] [--window-title TEXT] [--debug] [--log-file PATH] [--template PATH] [--no-startup] [--no-click-focus] [--no-control] [--use-shm] [--full-window] [--game-rect L,T,W,H] [--dry-run-frames N] [--list-windows]"
                );
                println!(
                    "Defaults are tuned for CachyOS/Linux + Dota 2 Dark Carnival Boot Catcher in windowed/XWayland mode at 60 FPS."
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }
    args
}

struct XContext {
    display: *mut Display,
    root: Window,
    screen: c_int,
}

impl XContext {
    fn open() -> io::Result<Self> {
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "could not open X11 display; set DISPLAY or run Dota 2 under X11/XWayland",
                ));
            }
            Ok(Self {
                display,
                root: XDefaultRootWindow(display),
                screen: XDefaultScreen(display),
            })
        }
    }

    fn window_title(&self, w: Window) -> Option<String> {
        self.get_window_property_string(w, "_NET_WM_NAME")
            .or_else(|| self.fetch_name(w))
            .or_else(|| self.get_window_property_string(w, "WM_NAME"))
    }

    fn fetch_name(&self, w: Window) -> Option<String> {
        unsafe {
            let mut name: *mut c_char = ptr::null_mut();
            if XFetchName(self.display, w, &mut name) == 0 || name.is_null() {
                return None;
            }
            let s = CStr::from_ptr(name).to_string_lossy().into_owned();
            XFree(name as *mut c_void);
            if s.is_empty() { None } else { Some(s) }
        }
    }

    fn get_window_property_string(&self, w: Window, prop_name: &str) -> Option<String> {
        unsafe {
            let prop_c = CString::new(prop_name).ok()?;
            let prop = XInternAtom(self.display, prop_c.as_ptr(), 1);
            if prop == 0 {
                return None;
            }
            let mut actual_type: Atom = 0;
            let mut actual_format: c_int = 0;
            let mut nitems: c_ulong = 0;
            let mut bytes_after: c_ulong = 0;
            let mut data: *mut c_uchar = ptr::null_mut();
            let status = XGetWindowProperty(
                self.display,
                w,
                prop,
                0,
                1024,
                0,
                ANY_PROPERTY_TYPE,
                &mut actual_type,
                &mut actual_format,
                &mut nitems,
                &mut bytes_after,
                &mut data,
            );
            if status != SUCCESS || data.is_null() || nitems == 0 {
                if !data.is_null() {
                    XFree(data as *mut c_void);
                }
                return None;
            }
            let byte_len = match actual_format {
                8 => nitems as usize,
                16 => nitems as usize * 2,
                32 => nitems as usize * 4,
                _ => 0,
            };
            let bytes = std::slice::from_raw_parts(data as *const u8, byte_len);
            let s = String::from_utf8_lossy(bytes)
                .trim_matches(char::from(0))
                .to_string();
            XFree(data as *mut c_void);
            if s.is_empty() { None } else { Some(s) }
        }
    }

    fn attributes(&self, w: Window) -> Option<XWindowAttributes> {
        unsafe {
            let mut attr: XWindowAttributes = std::mem::zeroed();
            if XGetWindowAttributes(self.display, w, &mut attr) == 0 {
                None
            } else {
                Some(attr)
            }
        }
    }

    fn rect_for_window(&self, w: Window) -> io::Result<Rect> {
        let attr = self
            .attributes(w)
            .ok_or_else(|| io::Error::last_os_error())?;
        let mut root_x = 0;
        let mut root_y = 0;
        let mut child: Window = 0;
        unsafe {
            XTranslateCoordinates(
                self.display,
                w,
                self.root,
                0,
                0,
                &mut root_x,
                &mut root_y,
                &mut child,
            );
        }
        Ok(Rect {
            left: root_x,
            top: root_y,
            width: attr.width,
            height: attr.height,
        })
    }

    fn collect_windows(&self, start: Window, out: &mut Vec<Window>) {
        unsafe {
            let mut root: Window = 0;
            let mut parent: Window = 0;
            let mut children: *mut Window = ptr::null_mut();
            let mut nchildren: c_uint = 0;
            if XQueryTree(
                self.display,
                start,
                &mut root,
                &mut parent,
                &mut children,
                &mut nchildren,
            ) == 0
            {
                return;
            }
            if !children.is_null() {
                let slice = std::slice::from_raw_parts(children, nchildren as usize);
                for &child in slice {
                    out.push(child);
                    self.collect_windows(child, out);
                }
                XFree(children as *mut c_void);
            }
        }
    }

    fn all_windows(&self) -> Vec<Window> {
        let mut out = Vec::new();
        self.collect_windows(self.root, &mut out);
        out
    }

    fn print_windows(&self) {
        for w in self.all_windows() {
            if let (Some(title), Some(attr)) = (self.window_title(w), self.attributes(w)) {
                if attr.width >= 200 && attr.height >= 100 {
                    println!(
                        "0x{w:x}\t{}x{}\tmap_state={}\t{}",
                        attr.width, attr.height, attr.map_state, title
                    );
                }
            }
        }
    }

    fn find_window_by_title(&self, needle: &str) -> Option<Window> {
        let needle = needle.to_lowercase();
        let mut best: Option<(Window, i32)> = None;
        for w in self.all_windows() {
            let Some(title) = self.window_title(w) else {
                continue;
            };
            if !title.to_lowercase().contains(&needle) {
                continue;
            }
            let Some(attr) = self.attributes(w) else {
                continue;
            };
            if attr.map_state != IS_VIEWABLE || attr.width < 300 || attr.height < 300 {
                continue;
            }
            let area = attr.width.saturating_mul(attr.height);
            if best.map_or(true, |(_, best_area)| area > best_area) {
                best = Some((w, area));
            }
        }
        best.map(|(w, _)| w)
    }
}

impl Drop for XContext {
    fn drop(&mut self) {
        unsafe {
            XCloseDisplay(self.display);
        }
    }
}

enum CaptureBackend {
    Shm {
        image: *mut XImage,
        shminfo: XShmSegmentInfo,
        marked_for_delete: bool,
    },
    XGetImage,
}

struct ScreenCapture {
    display: *mut Display,
    window: Window,
    src_x: i32,
    src_y: i32,
    width: i32,
    height: i32,
    buf: Vec<u8>,
    backend: CaptureBackend,
}

impl ScreenCapture {
    fn new(
        display: *mut Display,
        window: Window,
        attr: XWindowAttributes,
        src_x: i32,
        src_y: i32,
        width: i32,
        height: i32,
        use_shm: bool,
    ) -> io::Result<Self> {
        if width < 1 || height < 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "window size is invalid",
            ));
        }
        let backend = if use_shm {
            unsafe { create_shm_backend(display, window, attr, src_x, src_y, width, height) }
                .unwrap_or(CaptureBackend::XGetImage)
        } else {
            CaptureBackend::XGetImage
        };
        Ok(Self {
            display,
            window,
            src_x,
            src_y,
            width,
            height,
            buf: vec![0; (width * height * 4) as usize],
            backend,
        })
    }

    fn backend_name(&self) -> &'static str {
        match self.backend {
            CaptureBackend::Shm { .. } => "MIT-SHM",
            CaptureBackend::XGetImage => "XGetImage",
        }
    }

    fn grab_bgra(&mut self, _rect: Rect) -> io::Result<&[u8]> {
        unsafe {
            match &mut self.backend {
                CaptureBackend::Shm { image, .. } => {
                    if XShmGetImage(
                        self.display,
                        self.window,
                        *image,
                        self.src_x,
                        self.src_y,
                        !0,
                    ) == 0
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "XShmGetImage failed; is the Dota 2 window visible and not minimized?",
                        ));
                    }
                    copy_ximage_to_bgra(*image, self.width, self.height, &mut self.buf)?;
                }
                CaptureBackend::XGetImage => {
                    let image = XGetImage(
                        self.display,
                        self.window,
                        self.src_x,
                        self.src_y,
                        self.width as c_uint,
                        self.height as c_uint,
                        !0,
                        ZPIXMAP,
                    );
                    if image.is_null() {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "XGetImage failed; is the Dota 2 window visible and not minimized?",
                        ));
                    }
                    let result = copy_ximage_to_bgra(image, self.width, self.height, &mut self.buf);
                    XDestroyImage(image);
                    result?;
                }
            }
            Ok(&self.buf)
        }
    }
}

impl Drop for ScreenCapture {
    fn drop(&mut self) {
        unsafe {
            if let CaptureBackend::Shm {
                image,
                shminfo,
                marked_for_delete,
            } = &mut self.backend
            {
                XShmDetach(self.display, shminfo);
                XDestroyImage(*image);
                if !shminfo.shmaddr.is_null() {
                    shmdt(shminfo.shmaddr as *const c_void);
                }
                if !*marked_for_delete && shminfo.shmid >= 0 {
                    shmctl(shminfo.shmid, IPC_RMID, ptr::null_mut());
                }
            }
        }
    }
}

unsafe fn create_shm_backend(
    display: *mut Display,
    window: Window,
    attr: XWindowAttributes,
    src_x: i32,
    src_y: i32,
    width: i32,
    height: i32,
) -> Option<CaptureBackend> {
    if std::env::var_os("BOOT_CATCHER_DISABLE_SHM").is_some()
        || std::path::Path::new("/.dockerenv").exists()
    {
        return None;
    }
    if unsafe { XShmQueryExtension(display) } == 0 {
        return None;
    }
    let mut shminfo = XShmSegmentInfo {
        shmseg: 0,
        shmid: -1,
        shmaddr: ptr::null_mut(),
        read_only: 0,
    };
    let image = unsafe {
        XShmCreateImage(
            display,
            attr.visual,
            attr.depth as c_uint,
            ZPIXMAP,
            ptr::null_mut(),
            &mut shminfo,
            width as c_uint,
            height as c_uint,
        )
    };
    if image.is_null() {
        return None;
    }
    let img = unsafe { &mut *image };
    let size = (img.bytes_per_line as usize).saturating_mul(img.height as usize);
    if size == 0 {
        unsafe { XDestroyImage(image) };
        return None;
    }
    shminfo.shmid = unsafe { shmget(IPC_PRIVATE, size, IPC_CREAT | 0o600) };
    if shminfo.shmid < 0 {
        unsafe { XDestroyImage(image) };
        return None;
    }
    let addr = unsafe { shmat(shminfo.shmid, ptr::null(), 0) };
    if (addr as isize) == -1 {
        unsafe {
            shmctl(shminfo.shmid, IPC_RMID, ptr::null_mut());
            XDestroyImage(image);
        }
        return None;
    }
    shminfo.shmaddr = addr as *mut c_char;
    img.data = shminfo.shmaddr;
    shminfo.read_only = 0;
    X_ERROR_SEEN.store(false, Ordering::SeqCst);
    let previous_handler = unsafe { XSetErrorHandler(Some(x_error_handler)) };
    let attach_ok = unsafe { XShmAttach(display, &mut shminfo) } != 0;
    unsafe { XSync(display, 0) };
    let attach_failed = !attach_ok || X_ERROR_SEEN.load(Ordering::SeqCst);

    let probe_ok = if attach_failed {
        false
    } else {
        X_ERROR_SEEN.store(false, Ordering::SeqCst);
        let ok = unsafe { XShmGetImage(display, window, image, src_x, src_y, !0) } != 0;
        unsafe { XSync(display, 0) };
        ok && !X_ERROR_SEEN.load(Ordering::SeqCst)
    };
    unsafe { XSetErrorHandler(previous_handler) };

    if !probe_ok {
        unsafe {
            if attach_ok {
                XShmDetach(display, &mut shminfo);
            }
            shmdt(shminfo.shmaddr as *const c_void);
            shmctl(shminfo.shmid, IPC_RMID, ptr::null_mut());
            XDestroyImage(image);
        }
        return None;
    }
    Some(CaptureBackend::Shm {
        image,
        shminfo,
        marked_for_delete: false,
    })
}

unsafe fn copy_ximage_to_bgra(
    image: *mut XImage,
    width: i32,
    height: i32,
    out: &mut [u8],
) -> io::Result<()> {
    let img = unsafe { &*image };
    if img.data.is_null() {
        return Err(io::Error::new(io::ErrorKind::Other, "XImage data was null"));
    }
    let bytes_per_pixel = (img.bits_per_pixel / 8).max(1) as usize;
    if !(img.bits_per_pixel == 32 || img.bits_per_pixel == 24 || img.bits_per_pixel == 16) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "unsupported XImage depth/bpp: depth={} bpp={}",
                img.depth, img.bits_per_pixel
            ),
        ));
    }
    for y in 0..height as usize {
        let row = unsafe { (img.data as *const u8).add(y * img.bytes_per_line as usize) };
        for x in 0..width as usize {
            let p = unsafe { row.add(x * bytes_per_pixel) };
            let pixel = match img.bits_per_pixel {
                32 => {
                    let b = unsafe { std::slice::from_raw_parts(p, 4) };
                    if img.byte_order == LSB_FIRST {
                        u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as c_ulong
                    } else {
                        u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as c_ulong
                    }
                }
                24 => {
                    let b = unsafe { std::slice::from_raw_parts(p, 3) };
                    if img.byte_order == LSB_FIRST {
                        u32::from_le_bytes([b[0], b[1], b[2], 0]) as c_ulong
                    } else {
                        u32::from_be_bytes([0, b[0], b[1], b[2]]) as c_ulong
                    }
                }
                16 => {
                    let b = unsafe { std::slice::from_raw_parts(p, 2) };
                    if img.byte_order == LSB_FIRST {
                        u16::from_le_bytes([b[0], b[1]]) as c_ulong
                    } else {
                        u16::from_be_bytes([b[0], b[1]]) as c_ulong
                    }
                }
                _ => unreachable!(),
            };
            let r = extract_channel(pixel, img.red_mask);
            let g = extract_channel(pixel, img.green_mask);
            let b = extract_channel(pixel, img.blue_mask);
            let oi = (y * width as usize + x) * 4;
            out[oi] = b;
            out[oi + 1] = g;
            out[oi + 2] = r;
            out[oi + 3] = 255;
        }
    }
    Ok(())
}

fn extract_channel(pixel: c_ulong, mask: c_ulong) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let bits = mask.count_ones();
    let raw = (pixel & mask) >> shift;
    let max = (1u64 << bits) - 1;
    ((raw as u64 * 255 + max / 2) / max) as u8
}

struct XInput {
    display: *mut Display,
    screen: c_int,
    key_a: c_uint,
    key_d: c_uint,
    key_space: c_uint,
}

thread_local! {
    static XINPUT: std::cell::RefCell<Option<XInput>> = const { std::cell::RefCell::new(None) };
}

impl XInput {
    fn open() -> io::Result<Self> {
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "could not open X11 display for input",
                ));
            }
            let key_a = XKeysymToKeycode(display, XK_A);
            let key_d = XKeysymToKeycode(display, XK_D);
            let key_space = XKeysymToKeycode(display, XK_SPACE);
            if key_a == 0 || key_d == 0 || key_space == 0 {
                XCloseDisplay(display);
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "could not resolve X11 keycodes for A/D/Space",
                ));
            }
            Ok(Self {
                display,
                screen: XDefaultScreen(display),
                key_a,
                key_d,
                key_space,
            })
        }
    }

    fn keycode(&self, key: KeySym) -> c_uint {
        match key {
            XK_A => self.key_a,
            XK_D => self.key_d,
            XK_SPACE => self.key_space,
            _ => unsafe { XKeysymToKeycode(self.display, key) },
        }
    }

    fn key_event(&self, key: KeySym, up: bool) {
        unsafe {
            let keycode = self.keycode(key);
            let pressed = if up { 0 } else { 1 };
            XTestFakeKeyEvent(self.display, keycode, pressed, 0);
            XFlush(self.display);
        }
    }

    fn click_focus(&self, window: Window, rect: Rect) {
        unsafe {
            XRaiseWindow(self.display, window);
            XSetInputFocus(self.display, window, REVERT_TO_PARENT, CURRENT_TIME);
            let x = rect.left + rect.width / 2;
            let y = rect.top + (rect.height as f32 * 0.82) as i32;
            XTestFakeMotionEvent(self.display, self.screen, x, y, 0);
            XFlush(self.display);
            sleep(Duration::from_millis(40));
            XTestFakeButtonEvent(self.display, 1, 1, 0);
            XFlush(self.display);
            sleep(Duration::from_millis(30));
            XTestFakeButtonEvent(self.display, 1, 0, 0);
            XFlush(self.display);
        }
        sleep(Duration::from_millis(120));
    }
}

impl Drop for XInput {
    fn drop(&mut self) {
        unsafe {
            XCloseDisplay(self.display);
        }
    }
}

fn install_input(input: XInput) {
    XINPUT.with(|cell| {
        *cell.borrow_mut() = Some(input);
    });
}

fn key_event(key: KeySym, up: bool) {
    XINPUT.with(|cell| {
        if let Some(input) = cell.borrow().as_ref() {
            input.key_event(key, up);
        } else {
            eprintln!("X11 input backend is not initialized");
        }
    });
}

fn click_game(window: Window, rect: Rect) {
    XINPUT.with(|cell| {
        if let Some(input) = cell.borrow().as_ref() {
            input.click_focus(window, rect);
        }
    });
}

fn release_keys(held: &mut HeldKey, force_all: bool) {
    match *held {
        HeldKey::A => key_event(VK_A, true),
        HeldKey::D => key_event(VK_D, true),
        HeldKey::None if force_all => {
            key_event(VK_A, true);
            key_event(VK_D, true);
        }
        HeldKey::None => {}
    }
    *held = HeldKey::None;
}

fn hold_key(held: &mut HeldKey, key: HeldKey) {
    if *held == key {
        return;
    }
    release_keys(held, false);
    match key {
        HeldKey::A => key_event(VK_A, false),
        HeldKey::D => key_event(VK_D, false),
        HeldKey::None => {}
    }
    *held = key;
}

fn read_png_size(path: &str) -> io::Result<BootSize> {
    let mut f = File::open(path)?;
    let mut header = [0u8; 24];
    f.read_exact(&mut header)?;
    let png_sig = [137, 80, 78, 71, 13, 10, 26, 10];
    if header[0..8] != png_sig || &header[12..16] != b"IHDR" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "template must be PNG",
        ));
    }
    let w = i32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let h = i32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    Ok(BootSize { w, h })
}

fn clamp_f32(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi)
}

fn base_search_roi(width: i32, height: i32, args: &Args) -> Roi {
    let x0 = (width as f32 * args.search_margin) as i32;
    let x1 = (width as f32 * (1.0 - args.search_margin)) as i32;
    let y0 = (height as f32 * args.search_top) as i32;
    let y1 = (height as f32 * args.search_bottom) as i32;
    Roi {
        x: x0,
        y: y0,
        w: (x1 - x0).max(1),
        h: (y1 - y0).max(1),
    }
}

fn intersect(a: Roi, b: Roi) -> Option<Roi> {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = (a.x + a.w).min(b.x + b.w);
    let y1 = (a.y + a.h).min(b.y + b.h);
    if x1 <= x0 + 8 || y1 <= y0 + 8 {
        None
    } else {
        Some(Roi {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
        })
    }
}

fn roi_around(x: f32, y: f32, radius: i32, width: i32, height: i32, base: Roi) -> Roi {
    let local = Roi {
        x: (x - radius as f32).round() as i32,
        y: (y - radius as f32).round() as i32,
        w: radius * 2,
        h: radius * 2,
    };
    let screen = Roi {
        x: 0,
        y: 0,
        w: width,
        h: height,
    };
    intersect(local, screen)
        .and_then(|r| intersect(r, base))
        .unwrap_or(base)
}

fn idx(width: i32, x: i32, y: i32) -> usize {
    ((y * width + x) * 4) as usize
}

fn bgra_to_gray(frame: &[u8], width: i32, height: i32, out: &mut Vec<u8>) {
    out.resize((width * height) as usize, 0);
    for y in 0..height {
        for x in 0..width {
            let i = idx(width, x, y);
            let b = frame[i] as u32;
            let g = frame[i + 1] as u32;
            let r = frame[i + 2] as u32;
            out[(y * width + x) as usize] = ((77 * r + 150 * g + 29 * b) >> 8) as u8;
        }
    }
}

fn hsv_like(r: u8, g: u8, b: u8) -> (i32, i32, i32) {
    let r = r as i32;
    let g = g as i32;
    let b = b as i32;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let s = if max == 0 { 0 } else { d * 255 / max };
    let mut h = if d == 0 {
        0
    } else if max == r {
        30 * (g - b) / d
    } else if max == g {
        60 + 30 * (b - r) / d
    } else {
        120 + 30 * (r - g) / d
    };
    if h < 0 {
        h += 180;
    }
    (h, s, max)
}

fn boot_color(frame: &[u8], width: i32, x: i32, y: i32) -> bool {
    let i = idx(width, x, y);
    let (h, s, v) = hsv_like(frame[i + 2], frame[i + 1], frame[i]);
    (3..=36).contains(&h) && s >= 35 && v >= 30 && v <= 245
}

fn red_color(frame: &[u8], width: i32, x: i32, y: i32) -> bool {
    let i = idx(width, x, y);
    let (h, s, v) = hsv_like(frame[i + 2], frame[i + 1], frame[i]);
    ((0..=20).contains(&h) || (150..=179).contains(&h)) && s >= 45 && v >= 35
}

fn find_cart_center(frame: &[u8], width: i32, height: i32) -> Option<(i32, i32)> {
    let y0 = (height as f32 * 0.79) as i32;
    let y1 = (height as f32 * 0.91) as i32;
    let margin = (width as f32 * 0.08) as i32;
    let mut col_counts = vec![0i32; width as usize];
    for y in y0..y1 {
        for x in margin..(width - margin) {
            if red_color(frame, width, x, y) {
                col_counts[x as usize] += 1;
            }
        }
    }
    let mut best: Option<(i32, i32, i32)> = None;
    let mut start: Option<i32> = None;
    for x in 0..width {
        let active = col_counts[x as usize] >= 3;
        match (active, start) {
            (true, None) => start = Some(x),
            (false, Some(s)) => {
                let len = x - s;
                if len >= (width as f32 * 0.08) as i32 && len <= (width as f32 * 0.34) as i32 {
                    let sum: i32 = col_counts[s as usize..x as usize].iter().sum();
                    if best.map_or(true, |b| sum > b.2) {
                        best = Some((s, x, sum));
                    }
                }
                start = None;
            }
            _ => {}
        }
    }
    let (x0, x1, _) = best?;
    Some(((x0 + x1) / 2, (height as f32 * 0.88) as i32))
}

fn detect_boot_motion(
    frame: &[u8],
    gray: &[u8],
    prev_gray: Option<&[u8]>,
    width: i32,
    roi: Roi,
    boot_size: BootSize,
    max_scale: f32,
    source: &'static str,
    expected: Option<(f32, f32)>,
) -> Option<Detection> {
    let prev = prev_gray?;
    let max_w = (boot_size.w as f32 * max_scale) as i32;
    let max_h = (boot_size.h as f32 * max_scale) as i32;
    let mut visited = vec![false; (roi.w * roi.h) as usize];
    let mut best: Option<Detection> = None;
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];

    for yy in 0..roi.h {
        for xx in 0..roi.w {
            let local_i = (yy * roi.w + xx) as usize;
            if visited[local_i] {
                continue;
            }
            let x = roi.x + xx;
            let y = roi.y + yy;
            let gi = (y * width + x) as usize;
            let motion = (gray[gi] as i32 - prev[gi] as i32).abs() >= 18;
            if !motion || !boot_color(frame, width, x, y) {
                visited[local_i] = true;
                continue;
            }

            let mut stack = vec![(xx, yy)];
            visited[local_i] = true;
            let mut min_x = xx;
            let mut max_x = xx;
            let mut min_y = yy;
            let mut max_y = yy;
            let mut count = 0i32;

            while let Some((cx, cy)) = stack.pop() {
                count += 1;
                min_x = min_x.min(cx);
                max_x = max_x.max(cx);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);
                for (dx, dy) in dirs {
                    let nx = cx + dx;
                    let ny = cy + dy;
                    if nx < 0 || ny < 0 || nx >= roi.w || ny >= roi.h {
                        continue;
                    }
                    let ni = (ny * roi.w + nx) as usize;
                    if visited[ni] {
                        continue;
                    }
                    let px = roi.x + nx;
                    let py = roi.y + ny;
                    let pgi = (py * width + px) as usize;
                    let is_motion = (gray[pgi] as i32 - prev[pgi] as i32).abs() >= 18;
                    if is_motion && boot_color(frame, width, px, py) {
                        visited[ni] = true;
                        stack.push((nx, ny));
                    } else {
                        visited[ni] = true;
                    }
                }
            }

            let w = max_x - min_x + 1;
            let h = max_y - min_y + 1;
            if count < 35 || w < 8 || h < 8 || w > max_w || h > max_h {
                continue;
            }
            let aspect = w as f32 / h.max(1) as f32;
            if !(0.25..=3.2).contains(&aspect) {
                continue;
            }
            let fill = count as f32 / (w * h).max(1) as f32;
            let size_score = (count as f32 / 260.0).min(1.0);
            let shape_score = 1.0 - ((aspect - 0.85).abs() / 2.2).min(1.0);
            let cx_abs = roi.x + (min_x + max_x) / 2;
            let cy_abs = roi.y + (min_y + max_y) / 2;
            let distance_penalty = expected.map_or(0.0, |(ex, ey)| {
                let dx = cx_abs as f32 - ex;
                let dy = cy_abs as f32 - ey;
                let diag = ((roi.w * roi.w + roi.h * roi.h) as f32).sqrt().max(1.0);
                ((dx * dx + dy * dy).sqrt() / diag).min(1.0) * 0.55
            });
            let score = 0.40 * fill + 0.45 * size_score + 0.15 * shape_score - distance_penalty;
            if best.map_or(true, |b| score > b.score) {
                best = Some(Detection {
                    x: cx_abs,
                    y: cy_abs,
                    score,
                    bbox: Roi {
                        x: roi.x + min_x,
                        y: roi.y + min_y,
                        w,
                        h,
                    },
                    source,
                });
            }
        }
    }
    best
}

fn simulate_motion(
    mut s: MotionState,
    target_t: f64,
    width: i32,
    height: i32,
    args: &Args,
) -> MotionState {
    let left = width as f32 * args.search_margin + args.boot_radius as f32;
    let right = width as f32 * (1.0 - args.search_margin) - args.boot_radius as f32;
    let top = height as f32 * args.search_top + args.boot_radius as f32;
    let bottom = height as f32 * args.search_bottom - args.boot_radius as f32;
    let dt_total = (target_t - s.last_t).max(0.0) as f32;
    let step = args.physics_step.clamp(0.004, 0.05);
    let steps = (dt_total / step).ceil().max(1.0) as i32;
    let dt = if steps > 0 {
        dt_total / steps as f32
    } else {
        0.0
    };
    for _ in 0..steps {
        s.x += s.vx * dt;
        s.y += s.vy * dt;
        if s.x < left {
            s.x = left + (left - s.x);
            s.vx = s.vx.abs();
        } else if s.x > right {
            s.x = right - (s.x - right);
            s.vx = -s.vx.abs();
        }
        if s.y < top {
            s.y = top + (top - s.y);
            s.vy = s.vy.abs();
        } else if s.y > bottom {
            s.y = bottom - (s.y - bottom);
            s.vy = -s.vy.abs();
        }
        s.x = clamp_f32(s.x, left, right);
        s.y = clamp_f32(s.y, top, bottom);
    }
    s.last_t = target_t;
    s
}

fn trajectory_rois(
    pred: MotionState,
    width: i32,
    height: i32,
    base: Roi,
    boot_size: BootSize,
    args: &Args,
) -> Vec<Roi> {
    let base_radius = (((boot_size.w * boot_size.w + boot_size.h * boot_size.h) as f32).sqrt()
        / 2.0)
        .ceil() as i32
        + args.roi_padding;
    let radius = (base_radius as f32
        + pred.speed() * args.roi_speed
        + pred.lost_frames as f32 * args.lost_roi_grow as f32) as i32;
    let horizon =
        (0.18 + pred.lost_frames as f32 * 0.08).clamp(0.18, args.intercept_horizon.min(1.0));
    let step = (args.physics_step * 2.0).clamp(0.025, 0.08);
    let mut rois = Vec::new();
    let mut sim = pred;
    let now = pred.last_t;
    let mut t = 0.0f32;
    rois.push(roi_around(sim.x, sim.y, radius, width, height, base));
    while t < horizon {
        t += step;
        sim = simulate_motion(sim, now + step as f64, width, height, args);
        sim.last_t = now;
        rois.push(roi_around(sim.x, sim.y, radius, width, height, base));
    }
    rois
}

fn update_boot(
    state: &mut Option<MotionState>,
    prev_det: &mut Option<(f64, f32, f32)>,
    det: Detection,
    now: f64,
) {
    let (vx, vy, seen) = if let Some((pt, px, py)) = *prev_det {
        let dt = (now - pt).max(1e-4) as f32;
        let raw_vx = (det.x as f32 - px) / dt;
        let raw_vy = (det.y as f32 - py) / dt;
        if let Some(s) = *state {
            let alpha = 0.45;
            (
                s.vx * (1.0 - alpha) + raw_vx * alpha,
                s.vy * (1.0 - alpha) + raw_vy * alpha,
                s.seen_frames + 1,
            )
        } else {
            (raw_vx, raw_vy, 1)
        }
    } else {
        (0.0, 0.0, 1)
    };
    *state = Some(MotionState {
        x: det.x as f32,
        y: det.y as f32,
        vx,
        vy,
        last_t: now,
        lost_frames: 0,
        seen_frames: seen,
        score: det.score,
    });
    *prev_det = Some((now, det.x as f32, det.y as f32));
}

fn mark_lost(state: &mut Option<MotionState>, now: f64, width: i32, height: i32, args: &Args) {
    if let Some(s) = *state {
        let mut pred = simulate_motion(s, now, width, height, args);
        pred.lost_frames = s.lost_frames + 1;
        pred.score *= 0.92;
        *state = Some(pred);
    }
}

fn accept_boot_detection(state: Option<MotionState>, det: Detection, now: f64) -> bool {
    let Some(s) = state else {
        return true;
    };
    if s.seen_frames < 2 || s.lost_frames > 6 {
        return true;
    }
    let dt = (now - s.last_t).max(0.0) as f32;
    let expected_x = s.x + s.vx * dt;
    let expected_y = s.y + s.vy * dt;
    let dx = det.x as f32 - expected_x;
    let dy = det.y as f32 - expected_y;
    let dist = (dx * dx + dy * dy).sqrt();
    let gate = 150.0 + s.speed() * 0.12 + s.lost_frames as f32 * 45.0;
    dist <= gate || det.score >= 0.82
}

fn update_cart(
    state: &mut Option<MotionState>,
    det: Option<(i32, i32)>,
    now: f64,
    width: i32,
    height: i32,
    key: HeldKey,
    args: &Args,
) -> MotionState {
    let fallback_y = height as f32 * args.cart_y;
    match (det, *state) {
        (Some((x, y)), Some(s)) => {
            let dt = (now - s.last_t).max(1e-4) as f32;
            let raw_vx = (x as f32 - s.x) / dt;
            let vx = s.vx * 0.5 + raw_vx * 0.5;
            let next = MotionState {
                x: x as f32,
                y: y as f32,
                vx,
                vy: 0.0,
                last_t: now,
                lost_frames: 0,
                seen_frames: s.seen_frames + 1,
                score: 1.0,
            };
            *state = Some(next);
            next
        }
        (Some((x, y)), None) => {
            let next = MotionState {
                x: x as f32,
                y: y as f32,
                vx: 0.0,
                vy: 0.0,
                last_t: now,
                lost_frames: 0,
                seen_frames: 1,
                score: 1.0,
            };
            *state = Some(next);
            next
        }
        (None, Some(s)) => {
            let dt = (now - s.last_t).max(0.0) as f32;
            let vx = match key {
                HeldKey::A => -args.cart_speed.abs(),
                HeldKey::D => args.cart_speed.abs(),
                HeldKey::None => s.vx * 0.82,
            };
            let x = clamp_f32(s.x + vx * dt, 0.0, width as f32);
            let next = MotionState {
                x,
                y: s.y,
                vx,
                vy: 0.0,
                last_t: now,
                lost_frames: s.lost_frames + 1,
                seen_frames: s.seen_frames,
                score: s.score * 0.95,
            };
            *state = Some(next);
            next
        }
        (None, None) => {
            let next = MotionState {
                x: width as f32 / 2.0,
                y: fallback_y,
                vx: 0.0,
                vy: 0.0,
                last_t: now,
                lost_frames: 1,
                seen_frames: 0,
                score: 0.0,
            };
            *state = Some(next);
            next
        }
    }
}

fn steer_to(
    target_x: f32,
    cart: MotionState,
    intercept_t: f32,
    held: &mut HeldKey,
    args: &Args,
) -> (char, f32) {
    let lookahead = (intercept_t * 0.35).clamp(0.0, 0.18);
    let error = target_x - (cart.x + cart.vx * lookahead);
    let switch_zone = args.deadzone * 1.45;
    let release_zone = args.deadzone * 0.55;
    match *held {
        HeldKey::D => {
            if error < -switch_zone {
                hold_key(held, HeldKey::A);
                ('A', error)
            } else if error > release_zone {
                ('D', error)
            } else {
                release_keys(held, false);
                ('-', error)
            }
        }
        HeldKey::A => {
            if error > switch_zone {
                hold_key(held, HeldKey::D);
                ('D', error)
            } else if error < -release_zone {
                ('A', error)
            } else {
                release_keys(held, false);
                ('-', error)
            }
        }
        HeldKey::None => {
            if error > args.deadzone {
                hold_key(held, HeldKey::D);
                ('D', error)
            } else if error < -args.deadzone {
                hold_key(held, HeldKey::A);
                ('A', error)
            } else {
                ('-', error)
            }
        }
    }
}

fn run_startup() {
    key_event(VK_D, false);
    sleep(Duration::from_millis(600));
    key_event(VK_D, true);
    for _ in 0..2 {
        key_event(VK_SPACE, false);
        sleep(Duration::from_millis(250));
        key_event(VK_SPACE, true);
        sleep(Duration::from_millis(180));
    }
    key_event(VK_A, false);
    sleep(Duration::from_millis(600));
    key_event(VK_A, true);
    for _ in 0..2 {
        key_event(VK_SPACE, false);
        sleep(Duration::from_millis(250));
        key_event(VK_SPACE, true);
        sleep(Duration::from_millis(180));
    }
}

fn open_log(path: &str) -> io::Result<Option<BufWriter<File>>> {
    if path.is_empty() {
        return Ok(None);
    }
    let mut w = BufWriter::new(File::create(path)?);
    writeln!(
        w,
        "frame\ttime\tboot_det_x\tboot_det_y\tboot_score\tboot_source\tboot_box\tboot_track_x\tboot_track_y\tboot_vx\tboot_vy\tboot_lost\tcart_det_x\tcart_det_y\tcart_track_x\tcart_track_y\tcart_vx\tcart_lost\ttarget_x\terror\taction\tkey\treject_reason\tloop_ms\tfps\tboot_expected_w\tboot_expected_h\tsearch_roi_count\tsearch_roi_area\tsearch_rois\tgrab_ms\tgray_ms\tcart_ms\tboot_search_ms\tintercept_ms\tcontrol_ms\tlog_ms"
    )?;
    Ok(Some(w))
}

fn main() -> io::Result<()> {
    unsafe {
        signal(2, signal_handler);
        signal(15, signal_handler);
    }

    let args = parse_args();
    let x = XContext::open()?;

    if args.list_windows {
        x.print_windows();
        return Ok(());
    }

    let window = x.find_window_by_title(&args.window_title).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("could not find a visible X11/XWayland window matching {:?}; run with --list-windows or pass --window-title", args.window_title),
        )
    })?;
    let title = x
        .window_title(window)
        .unwrap_or_else(|| "<untitled>".to_string());
    let window_rect = x.rect_for_window(window)?;
    if window_rect.width < 250 || window_rect.height < 350 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Dota/window rect is too small: {window_rect:?}"),
        ));
    }
    let local_capture_rect = if args.full_window {
        Rect {
            left: 0,
            top: 0,
            width: window_rect.width,
            height: window_rect.height,
        }
    } else {
        clamp_rect_to_window(
            args.game_rect.unwrap_or_else(|| {
                default_dota_boot_catcher_rect(window_rect.width, window_rect.height)
            }),
            window_rect.width,
            window_rect.height,
        )
    };
    let rect = Rect {
        left: window_rect.left + local_capture_rect.left,
        top: window_rect.top + local_capture_rect.top,
        width: local_capture_rect.width,
        height: local_capture_rect.height,
    };

    install_input(XInput::open()?);

    if args.debug {
        println!("Using X11 window 0x{window:x}: {title}");
        println!(
            "Window rect: left={} top={} width={} height={}",
            window_rect.left, window_rect.top, window_rect.width, window_rect.height
        );
        println!(
            "Capture rect: window-relative left={} top={} width={} height={}",
            local_capture_rect.left,
            local_capture_rect.top,
            local_capture_rect.width,
            local_capture_rect.height
        );
    } else {
        println!(
            "Using window 0x{window:x}: {title}; capture {}x{}",
            rect.width, rect.height
        );
    }

    let boot_size = read_png_size(&args.template).unwrap_or(BootSize { w: 88, h: 112 });
    println!("Template size: {}x{}", boot_size.w, boot_size.h);
    println!(
        "Target FPS: {:.0}. Start in 3 seconds. Keep Dota 2 in windowed mode and visible.",
        args.fps
    );
    sleep(Duration::from_secs(3));
    if args.click_focus && args.control {
        click_game(window, rect);
    }
    if !args.no_startup && args.control {
        println!("Sending startup keys...");
        run_startup();
    }

    let win_attr = x.attributes(window).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "could not read Dota window attributes",
        )
    })?;
    let mut cap = ScreenCapture::new(
        x.display,
        window,
        win_attr,
        local_capture_rect.left,
        local_capture_rect.top,
        rect.width,
        rect.height,
        args.use_shm,
    )?;
    println!("Capture backend: {}", cap.backend_name());
    let mut gray = Vec::new();
    let mut prev_gray: Option<Vec<u8>> = None;
    let mut boot_state: Option<MotionState> = None;
    let mut prev_boot_det: Option<(f64, f32, f32)> = None;
    let mut last_found_boot_x: Option<f32> = None;
    let mut cart_state: Option<MotionState> = None;
    let mut held = HeldKey::None;
    let mut log = open_log(&args.log_file)?;
    let frame_delay = Duration::from_secs_f64(1.0 / args.fps.max(1.0));
    let start = Instant::now();
    let mut fps_est = 0.0f64;
    let mut frame_index = 0u64;
    let control_enabled = args.control;

    while !STOP.load(Ordering::SeqCst) {
        let loop_t0 = Instant::now();
        let now = start.elapsed().as_secs_f64();
        let t = Instant::now();
        let frame = cap.grab_bgra(rect)?;
        let grab_ms = t.elapsed().as_secs_f64() * 1000.0;

        let t = Instant::now();
        bgra_to_gray(frame, rect.width, rect.height, &mut gray);
        let gray_ms = t.elapsed().as_secs_f64() * 1000.0;

        let t = Instant::now();
        let cart_det = find_cart_center(frame, rect.width, rect.height);
        let cart = update_cart(
            &mut cart_state,
            cart_det,
            now,
            rect.width,
            rect.height,
            held,
            &args,
        );
        let cart_ms = t.elapsed().as_secs_f64() * 1000.0;

        let base_roi = base_search_roi(rect.width, rect.height, &args);
        let predicted = boot_state.map(|s| simulate_motion(s, now, rect.width, rect.height, &args));

        let t = Instant::now();
        let mut search_rois = Vec::new();
        let mut boot_det = None;
        if let Some(pred) = predicted {
            search_rois =
                trajectory_rois(pred, rect.width, rect.height, base_roi, boot_size, &args);
            for (i, roi) in search_rois.iter().copied().enumerate() {
                let src = if i == 0 {
                    "traj0-motion"
                } else {
                    "traj-motion"
                };
                boot_det = detect_boot_motion(
                    frame,
                    &gray,
                    prev_gray.as_deref(),
                    rect.width,
                    roi,
                    boot_size,
                    args.boot_size_max_scale,
                    src,
                    Some((pred.x, pred.y)),
                );
                if boot_det.is_some() {
                    break;
                }
            }
        } else {
            search_rois.push(base_roi);
            boot_det = detect_boot_motion(
                frame,
                &gray,
                prev_gray.as_deref(),
                rect.width,
                base_roi,
                boot_size,
                args.boot_size_max_scale,
                "full-init-motion",
                None,
            );
        }
        let boot_search_ms = t.elapsed().as_secs_f64() * 1000.0;

        let mut reject_reason = String::new();
        if let Some(det) = boot_det {
            let cart_line_y = cart_det.map_or(rect.height as f32 * args.cart_y, |(_, y)| y as f32);
            if det.y as f32 > cart_line_y + args.boot_radius as f32 {
                reject_reason = format!("boot_below_cart det_y={} cart_y={cart_line_y:.0}", det.y);
                boot_det = None;
            }
        }

        if let Some(det) = boot_det {
            if accept_boot_detection(boot_state, det, now) {
                last_found_boot_x = Some(det.x as f32);
                update_boot(&mut boot_state, &mut prev_boot_det, det, now);
            } else {
                reject_reason = "tracker_gate".to_string();
                boot_det = None;
                mark_lost(&mut boot_state, now, rect.width, rect.height, &args);
            }
        } else {
            mark_lost(&mut boot_state, now, rect.width, rect.height, &args);
            if boot_state.map_or(false, |s| s.lost_frames > args.lost_keep_frames) {
                boot_state = None;
                prev_boot_det = None;
                last_found_boot_x = None;
            }
        }

        let t = Instant::now();
        let target_x = last_found_boot_x;
        let intercept_t = 0.0;
        let intercept_ms = t.elapsed().as_secs_f64() * 1000.0;

        let t = Instant::now();
        let mut action = '-';
        let mut error = None;
        if let Some(boot) = boot_state {
            if !control_enabled {
                release_keys(&mut held, false);
                reject_reason = if reject_reason.is_empty() {
                    "control_disabled".to_string()
                } else {
                    reject_reason
                };
            } else if boot.lost_frames <= args.lost_keep_frames && target_x.is_some() {
                let (a, e) = steer_to(target_x.unwrap(), cart, intercept_t, &mut held, &args);
                action = a;
                error = Some(e);
            } else {
                release_keys(&mut held, false);
            }
        } else {
            release_keys(&mut held, false);
        }
        let control_ms = t.elapsed().as_secs_f64() * 1000.0;

        let elapsed = loop_t0.elapsed();
        let fps_now = 1.0 / elapsed.as_secs_f64().max(1e-6);
        fps_est = if fps_est <= 0.0 {
            fps_now
        } else {
            fps_est * 0.85 + fps_now * 0.15
        };

        if let Some(w) = log.as_mut() {
            let t = Instant::now();
            let roi_text = search_rois
                .iter()
                .map(|r| format!("{},{},{},{}", r.x, r.y, r.w, r.h))
                .collect::<Vec<_>>()
                .join(";");
            let roi_area: i32 = search_rois.iter().map(|r| r.w * r.h).sum();
            let bd = boot_det;
            let bs = boot_state;
            let row = vec![
                frame_index.to_string(),
                format!("{now:.6}"),
                bd.map_or(String::new(), |d| d.x.to_string()),
                bd.map_or(String::new(), |d| d.y.to_string()),
                bd.map_or(String::new(), |d| format!("{:.3}", d.score)),
                bd.map_or(String::new(), |d| d.source.to_string()),
                bd.map_or(String::new(), |d| {
                    format!("{},{},{},{}", d.bbox.x, d.bbox.y, d.bbox.w, d.bbox.h)
                }),
                bs.map_or(String::new(), |s| format!("{:.1}", s.x)),
                bs.map_or(String::new(), |s| format!("{:.1}", s.y)),
                bs.map_or(String::new(), |s| format!("{:.1}", s.vx)),
                bs.map_or(String::new(), |s| format!("{:.1}", s.vy)),
                bs.map_or(String::new(), |s| s.lost_frames.to_string()),
                cart_det.map_or(String::new(), |(x, _)| x.to_string()),
                cart_det.map_or(String::new(), |(_, y)| y.to_string()),
                format!("{:.1}", cart.x),
                format!("{:.1}", cart.y),
                format!("{:.1}", cart.vx),
                cart.lost_frames.to_string(),
                target_x.map_or(String::new(), |v| format!("{v:.1}")),
                error.map_or(String::new(), |v| format!("{v:.1}")),
                action.to_string(),
                format!("{held:?}"),
                reject_reason.clone(),
                format!("{:.2}", elapsed.as_secs_f64() * 1000.0),
                format!("{fps_est:.2}"),
                boot_size.w.to_string(),
                boot_size.h.to_string(),
                search_rois.len().to_string(),
                roi_area.to_string(),
                roi_text,
                format!("{grab_ms:.2}"),
                format!("{gray_ms:.2}"),
                format!("{cart_ms:.2}"),
                format!("{boot_search_ms:.2}"),
                format!("{intercept_ms:.2}"),
                format!("{control_ms:.2}"),
                format!("{:.2}", t.elapsed().as_secs_f64() * 1000.0),
            ];
            writeln!(w, "{}", row.join("\t"))?;
            if frame_index % 30 == 0 {
                w.flush()?;
            }
        }

        if args.debug && frame_index % 10 == 0 {
            println!(
                "frame={frame_index} fps={fps_est:.1} action={action} key={held:?} boot={:?} reject={}",
                boot_state.map(|s| (s.x as i32, s.y as i32, s.lost_frames)),
                reject_reason
            );
        }

        prev_gray = Some(gray.clone());
        frame_index += 1;
        if args
            .dry_run_frames
            .is_some_and(|limit| frame_index >= limit)
        {
            STOP.store(true, Ordering::SeqCst);
        }
        if elapsed < frame_delay {
            sleep(frame_delay - elapsed);
        }
    }
    release_keys(&mut held, true);
    if let Some(w) = log.as_mut() {
        w.flush()?;
    }
    println!("Stopped.");
    Ok(())
}
