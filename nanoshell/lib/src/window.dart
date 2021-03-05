import 'dart:async';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:pedantic/pedantic.dart';

import 'struts.dart';
import 'event.dart';
import 'menu.dart';
import 'util.dart';
import 'window_manager.dart';
import 'window_method_channel.dart';
import 'constants.dart';
import 'window_widget.dart';

class WindowHandle {
  const WindowHandle(this.value);

  final int value;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is WindowHandle && other.value == value);

  static const invalid = WindowHandle(-1);

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => 'WindowHandle($value)';
}

class Window {
  Window(this.handle);

  final WindowHandle handle;

  Future<void> show() async {
    if (_visible == true) {
      return;
    }
    _showCompleter ??= Completer();
    unawaited(_invokeMethod(Methods.windowShow));
    return _showCompleter!.future;
  }

  Future<dynamic> showModal() async {
    final res = await _invokeMethod(Methods.windowShowModal);
    return res;
  }

  Future<void> close() async {
    await _invokeMethod(Methods.windowClose);
  }

  Future<void> hide() async {
    _visible = null;
    await _invokeMethod(Methods.windowHide);
  }

  Future<GeometryFlags> setGeometry(Geometry request,
      [GeometryPreference preference =
          GeometryPreference.preferContent]) async {
    return GeometryFlags.deserialize(await _invokeMethod(
        Methods.windowSetGeometry, {
      'geometry': request.serialize(),
      'preference': enumToString(preference)
    }));
  }

  Future<Geometry> getGeometry() async {
    return Geometry.deserialize(await _invokeMethod(Methods.windowGetGeometry));
  }

  Future<GeometryFlags> supportedGeometry() async {
    return GeometryFlags.deserialize(
        await _invokeMethod(Methods.windowSupportedGeometry));
  }

  Future<void> setStyle(WindowStyle style) {
    return _invokeMethod(Methods.windowSetStyle, style.serialize());
  }

  static LocalWindow of(BuildContext context) =>
      WindowContext.of(context).window;

  static LocalWindow? maybeOf(BuildContext context) =>
      WindowContext.maybeoOf(context)?.window;

  static Window? fromHandle(WindowHandle handle) {
    return WindowManager.instance.getWindow(handle);
  }

  static Future<Window> create(dynamic initData) {
    return WindowManager.instance.createWindow(initData);
  }

  final visibilityChangedEvent = Event<bool>();
  final closeRequestEvent = VoidEvent();
  final closeEvent = VoidEvent();

  void onMessage(String message, dynamic arguments) {
    if (message == Events.windowInitialize) {
      _initialized = true;
      _initializedCompleter.complete();
    } else if (message == Events.windowVisibilityChanged) {
      _visible = arguments as bool;
      visibilityChangedEvent.fire(_visible!);
      if (_visible! && _showCompleter != null) {
        _showCompleter!.complete();
        _showCompleter = null;
      }
    } else if (message == Events.windowClose) {
      WindowManager.instance.windowClosed(this);
      closeEvent.fire();
    }
  }

  Future<dynamic> _invokeMethod(String method, [dynamic arguments]) {
    return WindowMethodDispatcher.instance.invokeMethod(
        channel: Channels.windowManager,
        method: method,
        arguments: arguments,
        targetWindowHandle: handle);
  }

  Future<void> waitUntilInitialized() async {
    if (_initialized) {
      return;
    } else {
      return _initializedCompleter.future;
    }
  }

  final _initializedCompleter = Completer<void>();
  Completer<void>? _showCompleter;
  bool? _visible;
  bool _initialized = false;
}

// Window that belongs to current isolate
class LocalWindow extends Window {
  LocalWindow(
    WindowHandle handle, {
    WindowHandle? parentWindow,
    this.initData,
  })  : _parentWindow = parentWindow,
        super(handle);

  @override
  void onMessage(String message, dynamic arguments) {
    if (message == Events.windowCloseRequest) {
      close();
    }
    super.onMessage(message, arguments);
  }

  @override
  Future<void> show() async {
    // Can't wait until readyToShow is called for window from current isolate;
    // that would cause deadlock
    return _invokeMethod(Methods.windowShow);
  }

  Future<void> readyToShow() async {
    await _invokeMethod(Methods.windowReadyToShow);
  }

  Future<PopupMenuResponse> showPopupMenu(
    Menu menu,
    Offset globalPosition, {
    Rect? trackingRect,
    Rect? itemRect,
    bool preselectFirst = false,
  }) async {
    return menu.materialize((handle) async {
      final value = await _invokeMethod(
          Methods.windowShowPopupMenu,
          PopupMenuRequest(
                  handle: handle,
                  position: globalPosition,
                  trackingRect: trackingRect,
                  itemRect: itemRect,
                  preselectFirst: preselectFirst)
              .serialize());
      return PopupMenuResponse.deserialize(value);
    });
  }

  Future<void> hidePopupMenu(MenuHandle handle) async {
    await _invokeMethod(Methods.windowHidePopupMenu,
        HidePopupMenuRequest(handle: handle).serialize());
  }

  Future<void> showSystemMenu() async {
    await _invokeMethod(Methods.windowShowSystemMenu);
  }

  Completer? _currentWindowMenuCompleter;

  Future<void> setWindowMenu(Menu menu) {
    final previousCompleter = _currentWindowMenuCompleter;

    final functionCompleter = Completer();
    final menuCompleter = Completer();
    _currentWindowMenuCompleter = menuCompleter;

    menu.materialize((handle) async {
      await _invokeMethod(Methods.windowSetWindowMenu, {
        'handle': menu.currentHandle!.value,
      });
      functionCompleter.complete();
      // keep the handle alive until completer
      return menuCompleter.future;
    });

    if (previousCompleter != null) {
      previousCompleter.complete();
    }

    return functionCompleter.future;
  }

  Future<void> performDrag() async {
    await _invokeMethod(Methods.windowPerformDrag);
  }

  Future<void> closeWithResult(dynamic result) async {
    await _invokeMethod(Methods.windowCloseWithResult, result);
  }

  Window? get parentWindow =>
      WindowManager.instance.getWindow(_parentWindow ?? WindowHandle.invalid);

  final dynamic initData;
  final WindowHandle? _parentWindow;
}
