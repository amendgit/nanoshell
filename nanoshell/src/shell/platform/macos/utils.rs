use std::{os::raw::c_char, slice, sync::Arc};

use cocoa::{
    appkit::{CGFloat, NSImage},
    base::{id, nil},
    foundation::{NSArray, NSPoint, NSRect, NSSize, NSString},
};

use core_graphics::{
    base::{kCGBitmapByteOrderDefault, kCGImageAlphaLast, kCGRenderingIntentDefault},
    color_space::CGColorSpace,
    data_provider::CGDataProvider,
    image::CGImage,
};

use objc::{
    rc::StrongPtr,
    runtime::{Class, Object},
};

use crate::shell::{structs::ImageData, Point, Rect, Size};

impl<'a> From<&'a Size> for NSSize {
    fn from(size: &'a Size) -> Self {
        NSSize::new(size.width, size.height)
    }
}

impl<'a> From<&'a Point> for NSPoint {
    fn from(position: &'a Point) -> Self {
        NSPoint::new(position.x, position.y)
    }
}

impl From<Size> for NSSize {
    fn from(size: Size) -> Self {
        NSSize::new(size.width, size.height)
    }
}

impl From<Point> for NSPoint {
    fn from(position: Point) -> Self {
        NSPoint::new(position.x, position.y)
    }
}

impl<'a> From<&'a Rect> for NSRect {
    fn from(position: &'a Rect) -> Self {
        NSRect::new(position.origin().into(), position.size().into())
    }
}

impl From<Rect> for NSRect {
    fn from(position: Rect) -> Self {
        NSRect::new(position.origin().into(), position.size().into())
    }
}

impl From<NSSize> for Size {
    fn from(size: NSSize) -> Self {
        Size {
            width: size.width,
            height: size.height,
        }
    }
}

impl From<NSPoint> for Point {
    fn from(point: NSPoint) -> Self {
        Point {
            x: point.x,
            y: point.y,
        }
    }
}

impl From<NSRect> for Rect {
    fn from(rect: NSRect) -> Self {
        Self::xywh(
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        )
    }
}

pub fn from_nsstring(ns_string: id) -> String {
    unsafe {
        let bytes: *const c_char = msg_send![ns_string, UTF8String];
        let bytes = bytes as *const u8;

        let len = ns_string.len();

        let bytes = slice::from_raw_parts(bytes, len);
        std::str::from_utf8(bytes).unwrap().into()
    }
}

pub fn to_nsstring(string: &str) -> StrongPtr {
    unsafe {
        let ptr = NSString::alloc(nil).init_str(string);
        StrongPtr::new(ptr)
    }
}

// pub fn from_nsdata(data: id) -> Vec<u8> {
//     unsafe {
//         let bytes: *const u8 = msg_send![data, bytes];
//         let length: usize = msg_send![data, length];
//         let data: &[u8] = std::slice::from_raw_parts(bytes, length);
//         data.into()
//     }
// }

pub fn to_nsdata(data: &[u8]) -> StrongPtr {
    unsafe {
        StrongPtr::retain(msg_send![class!(NSData), dataWithBytes:data.as_ptr() length:data.len()])
    }
}

pub unsafe fn superclass<'a>(this: &'a Object) -> &'a Class {
    let superclass: id = msg_send![this, superclass];
    &*(superclass as *const _)
}

pub unsafe fn array_with_objects(objects: &[StrongPtr]) -> id {
    let vec: Vec<id> = objects.iter().map(|f| *(f.clone()) as id).collect();
    NSArray::arrayWithObjects(nil, &vec)
}

pub fn ns_image_from(image: ImageData) -> StrongPtr {
    unsafe {
        let data = CGDataProvider::from_buffer(Arc::new(image.data));

        let rgb = CGColorSpace::create_device_rgb();

        let cgimage = CGImage::new(
            image.width as usize,
            image.height as usize,
            8,
            32,
            image.bytes_per_row as usize,
            &rgb,
            kCGBitmapByteOrderDefault | kCGImageAlphaLast,
            &data,
            true,
            kCGRenderingIntentDefault,
        );

        StrongPtr::new(msg_send![NSImage::alloc(nil),
            initWithCGImage:&*cgimage
            size:NSSize::new(image.width as CGFloat, image.height as CGFloat)
        ])
    }
}
