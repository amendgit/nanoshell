pub(crate) mod channel {
    // Flutter channel for windows bound messages; All messages that concern windows are
    // dispatched on this channel
    pub const DISPATCHER: &str = ".window.dispatcher";

    // Window sub channels (delivered by dispatcher)
    pub mod win {
        pub const WINDOW_MANAGER: &str = ".window.window-manager";
        pub const DROP_TARGET: &str = ".window.drop-target";
        pub const DRAG_SOURCE: &str = ".window.drag-source";
    }

    // Flutter channel for mananing platform menus
    pub const MENU_MANAGER: &str = ".menu-manager";
}

pub(crate) mod method {

    pub mod window {
        // Without targetWindowHandle, directed to Window Manager
        pub const CREATE: &str = "method:window-create";

        // Window targetWindowHandle, directed to Window Manager
        pub const INIT: &str = "method:window-init";

        // Request to show the window (may be delayed until window itself calls readyToShow)
        pub const SHOW: &str = "method:window-show";

        // Request to show the window modally (will return result after window closes)
        pub const SHOW_MODAL: &str = "method:window-show-modal";

        // Called by window itself after the layout is ready and window is prepared to be shown
        pub const READY_TO_SHOW: &str = "method:window-ready-to-show";

        // Hide the window
        pub const HIDE: &str = "method:window-hide";

        // Close the window; This will terminate the isolate
        pub const CLOSE: &str = "method:window-close";

        pub const CLOSE_WITH_RESULT: &str = "method:window-close-with-result";

        // All positions, unless otherwise noted are in logical coordinates with top left origin

        pub const SET_GEOMETRY: &str = "method:window-set-geometry";
        pub const GET_GEOMETRY: &str = "method:window-get-geometry";
        pub const SUPPORTED_GEOMETRY: &str = "method:window-supported-geometry";

        pub const SET_STYLE: &str = "method:window-set-style";
        pub const PERFORM_WINDOW_DRAG: &str = "method:window-perform-window-drag";

        pub const SHOW_POPUP_MENU: &str = "method:window-show-popup-menu";
    }

    pub mod drop_target {
        pub const DRAGGING_UPDATED: &str = "method:drop-target-dragging-updated";
        pub const DRAGGING_EXITED: &str = "method:drop-target-dragging-exited";
        pub const PERFORM_DROP: &str = "method:drop-target-perform-drop";
    }

    pub mod drag_source {
        pub const BEGIN_DRAG_SESSION: &str = "method:drag-source-begin-drag-session";
        pub const DRAG_SESSION_ENDED: &str = "method:drag-source-drag-session-ended";
    }

    pub mod menu {
        pub const CREATE_OR_UPDATE: &str = "method:menu-create-or-update";
        pub const DESTROY: &str = "method:menu-destroy";
        pub const ON_ACTION: &str = "method:menu-on-action";
    }

    pub mod menu_bar {
        // Menubar - move to previous menu
        pub const MOVE_TO_PREVIOUS_MENU: &str = "method:menubar-move-to-previous-menu";
        pub const MOVE_TO_NEXT_MENU: &str = "method:menubar-move-to-next-menu";
    }
}

pub(crate) mod event {
    pub mod window {
        // Called when window has been properly initialized and can receive messages
        pub const INITIALIZE: &str = "event:window-initialize";

        // Called when window became visible or hidden (boolean argument)
        pub const VISIBILITY_CHANGED: &str = "event:window-visibility-changed";

        // Delivered when user requested closing the window; Target window is responsible
        // for actually closing the window
        pub const CLOSE_REQUEST: &str = "event:window-close-request";

        // Delivered when window is actually closed
        pub const CLOSE: &str = "event:window-close";
    }
}

pub(crate) mod drag_data {
    pub mod key {
        pub const FILES: &str = "drag-data:internal:files";
        pub const URLS: &str = "drag-data:internal:urls";
    }
}
