import 'dart:async';

import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

import 'struts.dart';
import 'window.dart';
import 'window_manager.dart';

abstract class WindowBuilder {
  Widget build(BuildContext context);

  Future<void> initializeWindow(
      LocalWindow window, Size intrinsicContentSize) async {
    await window.setGeometry(Geometry(
      contentSize: intrinsicContentSize,
    ));
    await window.show();
  }

  bool get autoSizeWindow => false;
  Future<void> updateWindowSize(LocalWindow window, Size contentSize) async {
    await window.setGeometry(Geometry(contentSize: contentSize));
  }
}

typedef WindowBuilderProvider = WindowBuilder Function(dynamic initData);

class WindowWidget extends StatefulWidget {
  WindowWidget({
    required this.builder,
    Key? key,
  }) : super(key: key);

  final WindowBuilderProvider builder;

  @override
  State<StatefulWidget> createState() {
    return _WindowWidgetState();
  }
}

//
//
//

enum _Status { notInitialized, initializing, initialized }

class _WindowWidgetState extends State<WindowWidget> {
  @override
  Widget build(BuildContext context) {
    _maybeInitialize();
    if (status == _Status.initialized) {
      final window = WindowManager.instance.currentWindow;
      final build = widget.builder(window.initData);
      return LocalWindowWidget(
        child: _WindowLayout(
          builtWindow: build,
          child: _WindowLayoutInner(
            child: Builder(
              builder: (context) {
                return build.build(context);
              },
            ),
            builtWindow: build,
          ),
        ),
        window: window,
      );
    } else {
      return Container();
    }
  }

  void _maybeInitialize() async {
    if (status == _Status.notInitialized) {
      status = _Status.initializing;
      await WindowManager.initialize();
      status = _Status.initialized;
      setState(() {});
    }
  }

  _Status status = _Status.notInitialized;
  dynamic initData;
}

// Used by Window.of(context)
class LocalWindowWidget extends InheritedWidget {
  final LocalWindow window;

  LocalWindowWidget({
    required Widget child,
    required this.window,
  }) : super(child: child);

  @override
  bool updateShouldNotify(covariant InheritedWidget oldWidget) {
    return false;
  }
}

class _WindowLayoutInner extends SingleChildRenderObjectWidget {
  final WindowBuilder builtWindow;

  const _WindowLayoutInner({required Widget child, required this.builtWindow})
      : super(child: child);

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderWindowLayoutInner(builtWindow);
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant _RenderWindowLayoutInner renderObject) {
    renderObject.builtWindow = builtWindow;
  }
}

class _RenderWindowLayoutInner extends RenderProxyBox {
  _RenderWindowLayoutInner(this.builtWindow);

  WindowBuilder builtWindow;

  @override
  void performLayout() {
    if (!builtWindow.autoSizeWindow) {
      super.performLayout();
    } else {
      final constraints = this.constraints.loosen();
      child!.layout(constraints, parentUsesSize: true);
      assert(
          child!.size.width != constraints.maxWidth &&
              child!.size.height != constraints.maxHeight,
          "Child failed to constraint itself! If you're using Row or Column, "
          "don't forget to set mainAxisSize to MainAxisSize.min");
      size = child!.size;
      _updateGeometry();
    }
  }

  bool _geometryPending = false;
  bool _geometryInProgress = false;

  void _updateGeometry() async {
    if (_geometryInProgress) {
      _geometryPending = true;
    } else {
      _geometryInProgress = true;
      await builtWindow.updateWindowSize(
          WindowManager.instance.currentWindow, size);
      _geometryInProgress = false;
      if (_geometryPending) {
        _geometryPending = false;
        _updateGeometry();
      }
    }
  }
}

class _WindowLayout extends SingleChildRenderObjectWidget {
  final WindowBuilder builtWindow;

  const _WindowLayout({
    Key? key,
    required Widget child,
    required this.builtWindow,
  }) : super(key: key, child: child);

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderWindowLayout(builtWindow);
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant _RenderWindowLayout renderObject) {
    renderObject.builtWindow = builtWindow;
  }
}

class _RenderWindowLayout extends RenderProxyBox {
  _RenderWindowLayout(this.builtWindow);

  WindowBuilder builtWindow;

  @override
  void performLayout() {
    if (!hasLayout) {
      hasLayout = true;

      final win = WindowManager.instance.currentWindow;
      SchedulerBinding.instance!.scheduleFrameCallback((timeStamp) {
        SchedulerBinding.instance!.addPostFrameCallback((timeStamp) async {
          final w = child!.getMaxIntrinsicWidth(double.infinity);
          final h = child!.getMaxIntrinsicHeight(double.infinity);
          await builtWindow.initializeWindow(win, Size(w, h));
          await win.readyToShow();
        });
      });
    }

    if (builtWindow.autoSizeWindow) {
      final constraints =
          BoxConstraints.loose(Size(double.infinity, double.infinity));
      child!.layout(constraints, parentUsesSize: true);
      size = Size(this.constraints.maxWidth, this.constraints.maxHeight);
    } else {
      super.performLayout();
    }
  }

  bool hasLayout = false;
}
