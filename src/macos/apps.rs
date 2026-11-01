//! Running applications and their windows, via the window server.

use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation_sys::base::CFTypeRef;
use core_foundation_sys::dictionary::{CFDictionaryGetValue, CFDictionaryRef};

use super::cf;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CFArrayRef;
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
    static kCGWindowOwnerPID: CFTypeRef;
    static kCGWindowOwnerName: CFTypeRef;
    static kCGWindowName: CFTypeRef;
    static kCGWindowNumber: CFTypeRef;
    static kCGWindowLayer: CFTypeRef;
    static kCGWindowBounds: CFTypeRef;
}

const ON_SCREEN_ONLY: u32 = 1;
const EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;
const NULL_WINDOW: u32 = 0;

#[derive(Clone, Debug)]
pub struct Window {
    pub number: i64,
    pub title: String,
    pub layer: i64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug)]
pub struct App {
    pub pid: i32,
    pub name: String,
    pub windows: Vec<Window>,
}

pub fn screen_recording_allowed() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

pub fn request_screen_recording() -> bool {
    unsafe { CGRequestScreenCaptureAccess() }
}

fn dictionary_number(dict: CFDictionaryRef, key: CFTypeRef) -> Option<f64> {
    unsafe {
        let value = CFDictionaryGetValue(dict, key);
        cf::number(value as CFTypeRef).or_else(|| cf::integer(value as CFTypeRef).map(|n| n as f64))
    }
}

fn dictionary_string(dict: CFDictionaryRef, key: CFTypeRef) -> String {
    unsafe {
        let value = CFDictionaryGetValue(dict, key);
        cf::string_from(value as CFTypeRef).unwrap_or_default()
    }
}

fn bounds(dict: CFDictionaryRef) -> Option<(f64, f64, f64, f64)> {
    unsafe {
        let value = CFDictionaryGetValue(dict, kCGWindowBounds);
        if value.is_null() {
            return None;
        }
        let bounds = value as CFDictionaryRef;
        let x_key = cf::string("X")?;
        let y_key = cf::string("Y")?;
        let width_key = cf::string("Width")?;
        let height_key = cf::string("Height")?;
        Some((
            dictionary_number(bounds, x_key.raw())?,
            dictionary_number(bounds, y_key.raw())?,
            dictionary_number(bounds, width_key.raw())?,
            dictionary_number(bounds, height_key.raw())?,
        ))
    }
}

/// Every window the window server reports, grouped into applications.
pub fn running_apps() -> Vec<App> {
    let mut apps: Vec<App> = Vec::new();
    unsafe {
        let list =
            CGWindowListCopyWindowInfo(ON_SCREEN_ONLY | EXCLUDE_DESKTOP_ELEMENTS, NULL_WINDOW);
        let Some(owned) = cf::Owned::from_create(list as CFTypeRef) else {
            return apps;
        };
        let array = owned.raw() as CFArrayRef;
        for index in 0..CFArrayGetCount(array) {
            let entry = CFArrayGetValueAtIndex(array, index);
            if entry.is_null() {
                continue;
            }
            let entry = entry as CFDictionaryRef;
            let pid = match dictionary_number(entry, kCGWindowOwnerPID) {
                Some(pid) => pid as i32,
                None => continue,
            };
            let layer = dictionary_number(entry, kCGWindowLayer).unwrap_or(0.0) as i64;
            let name = dictionary_string(entry, kCGWindowOwnerName);
            let number = dictionary_number(entry, kCGWindowNumber).unwrap_or(0.0) as i64;
            let title = dictionary_string(entry, kCGWindowName);
            let (x, y, width, height) = bounds(entry).unwrap_or((0.0, 0.0, 0.0, 0.0));
            let window = Window {
                number,
                title,
                layer,
                x,
                y,
                width,
                height,
            };
            match apps.iter_mut().find(|app| app.pid == pid) {
                Some(app) => app.windows.push(window),
                None => apps.push(App {
                    pid,
                    name: if name.is_empty() {
                        format!("pid {pid}")
                    } else {
                        name
                    },
                    windows: vec![window],
                }),
            }
        }
    }
    apps
}

/// Normal (layer 0) windows owned by one process.
pub fn windows_for(pid: i32) -> Vec<Window> {
    running_apps()
        .into_iter()
        .filter(|app| app.pid == pid)
        .flat_map(|app| app.windows)
        .filter(|window| window.layer == 0)
        .collect()
}

/// Resolves `app` against the running applications: exact name first, then a
/// case-insensitive substring, then the process id when the value is numeric.
pub fn resolve(app: &str) -> Option<App> {
    let apps = running_apps();
    if let Ok(pid) = app.trim().parse::<i32>() {
        if let Some(found) = apps.iter().find(|candidate| candidate.pid == pid) {
            return Some(found.clone());
        }
    }
    let needle = app.trim().to_lowercase();
    apps.iter()
        .find(|candidate| candidate.name.to_lowercase() == needle)
        .or_else(|| {
            apps.iter()
                .find(|candidate| candidate.name.to_lowercase().contains(&needle))
        })
        .cloned()
}

pub fn app_names() -> Vec<String> {
    running_apps().into_iter().map(|app| app.name).collect()
}
