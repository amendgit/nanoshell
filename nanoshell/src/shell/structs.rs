use std::collections::HashMap;

use crate::codec::Value;

use super::{MenuHandle, Point, Rect, Size};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum GeometryPreference {
    PreferFrame,
    PreferContent,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometry {
    pub frame_origin: Option<Point>,
    pub frame_size: Option<Size>,
    pub content_origin: Option<Point>,
    pub content_size: Option<Size>,

    pub min_frame_size: Option<Size>,
    pub max_frame_size: Option<Size>,
    pub min_content_size: Option<Size>,
    pub max_content_size: Option<Size>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometryRequest {
    pub geometry: WindowGeometry,
    pub preference: GeometryPreference,
}

impl WindowGeometryRequest {
    // Returns geometry with redundand fields removed (useful when caller
    // supports all fields)
    pub fn filtered_by_preference(self) -> WindowGeometry {
        let mut geometry = self.geometry;

        match self.preference {
            GeometryPreference::PreferFrame => {
                if geometry.frame_origin.is_some() {
                    geometry.content_origin = None;
                }
                if geometry.frame_size.is_some() {
                    geometry.content_size = None;
                }
                if geometry.min_frame_size.is_some() {
                    geometry.min_content_size = None;
                }
                if geometry.max_frame_size.is_some() {
                    geometry.max_content_size = None;
                }
            }
            GeometryPreference::PreferContent => {
                if geometry.content_origin.is_some() {
                    geometry.frame_origin = None;
                }
                if geometry.content_size.is_some() {
                    geometry.frame_size = None;
                }
                if geometry.min_content_size.is_some() {
                    geometry.min_frame_size = None;
                }
                if geometry.max_content_size.is_some() {
                    geometry.max_frame_size = None;
                }
            }
        }

        geometry
    }
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            frame_origin: None,
            frame_size: None,
            content_origin: None,
            content_size: None,
            min_frame_size: None,
            max_frame_size: None,
            min_content_size: None,
            max_content_size: None,
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PopupMenuRequest {
    pub handle: MenuHandle,
    pub position: Point,

    // Windows only, used for menu bar implementation; is specified this
    // rect will keep receiving mouse events
    pub tracking_rect: Option<Rect>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometryFlags {
    pub frame_origin: bool,
    pub frame_size: bool,
    pub content_origin: bool,
    pub content_size: bool,
    pub min_frame_size: bool,
    pub max_frame_size: bool,
    pub min_content_size: bool,
    pub max_content_size: bool,
}

impl Default for WindowGeometryFlags {
    fn default() -> Self {
        Self {
            frame_origin: false,
            frame_size: false,
            content_origin: false,
            content_size: false,
            min_frame_size: false,
            max_frame_size: false,
            min_content_size: false,
            max_content_size: false,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DragData {
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum DragEffect {
    None,
    Copy,
    Link,
    Move,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DraggingInfo {
    pub location: Point,
    pub data: DragData,
    pub allowed_effects: Vec<DragEffect>,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DragResult {
    pub effect: DragEffect,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImageData {
    pub width: i32,
    pub height: i32,
    pub bytes_per_row: i32,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DragRequest {
    pub image: ImageData,
    pub rect: Rect,
    pub allowed_effects: Vec<DragEffect>,
    pub data: DragData,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum WindowFrame {
    Regular,
    NoTitle,
    NoFrame,
}

impl Default for WindowFrame {
    fn default() -> Self {
        WindowFrame::Regular
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowStyle {
    pub frame: WindowFrame,
    pub can_resize: bool,
    pub can_close: bool,
    pub can_minimize: bool,
    pub can_maximize: bool,
    pub can_full_screen: bool,
}
