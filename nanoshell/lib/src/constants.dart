class Channels {
  static final dispatcher = '.window.dispatcher';
  static final windowManager = '.window.window-manager';
  static final dropTarget = '.window.drop-target';
  static final dragSource = '.window.drag-source';
  static final menuManager = '.menu-manager';
}

class Events {
  static final windowInitialize = 'event:window-initialize';
  static final windowVisibilityChanged = 'event:window-visibility-changed';
  static final windowCloseRequest = 'event:window-close-request';
  static final windowClose = 'event:window-close';
}

class Methods {
  // Window
  static final windowCreate = 'method:window-create';
  static final windowInit = 'method:window-init';
  static final windowShow = 'method:window-show';
  static final windowShowModal = 'method:window-show-modal';
  static final windowReadyToShow = 'method:window-ready-to-show';
  static final windowHide = 'method:window-hide';
  static final windowClose = 'method:window-close';
  static final windowCloseWithResult = 'method:window-close-with-result';

  static final windowSetGeometry = 'method:window-set-geometry';
  static final windowGetGeometry = 'method:window-get-geometry';
  static final windowSupportedGeometry = 'method:window-supported-geometry';

  static final windowSetStyle = 'method:window-set-style';
  static final windowPerformDrag = 'method:window-perform-window-drag';

  static final windowShowPopupMenu = 'method:window-show-popup-menu';

  // Drop Target
  static final dropTargetDraggingUpdated =
      'method:drop-target-dragging-updated';
  static final dropTargetDraggingExited = 'method:drop-target-dragging-exited';
  static final dropTargetPerformDrop = 'method:drop-target-perform-drop';

  // Drop Source
  static final dragSourceBeginDragSession =
      'method:drag-source-begin-drag-session';
  static final dragSourceDragSessionEnded =
      'method:drag-source-drag-session-ended';

  // Menu
  static final menuCreateOrUpdate = 'method:menu-create-or-update';
  static final menuDestroy = 'method:menu-destroy';
  static final menuOnAction = 'method:menu-on-action';

  // Menubar
  static final menubarMoveToPreviousMenu =
      'method:menubar-move-to-previous-menu';
  static final menubarMoveToNextMenu = 'method:menubar-move-to-next-menu';
}

class Keys {
  static final dragDataFiles = 'drag-data:internal:files';
  static final dragDataURLs = 'drag-data:internal:urls';
}
