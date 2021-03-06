import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';
import 'package:nanoshell/nanoshell.dart';
import 'package:nanoshell_demo/file_open_dialog.dart';
import 'package:nanoshell_demo/util.dart';
import 'package:path/path.dart';

import 'drag_drop.dart';
import 'modal.dart';
import 'popup_menu.dart';
import 'veil.dart';

class HomeWindow extends WindowBuilder {
  @override
  Widget build(BuildContext context) {
    return Home();
  }

  @override
  bool get autoSizeWindow => true;

  @override
  Future<void> initializeWindow(
      LocalWindow window, Size intrinsicContentSize) async {
    await super.initializeWindow(window, intrinsicContentSize);
    await window.setStyle(WindowStyle(canResize: false));
  }
}

class Home extends StatefulWidget {
  @override
  State<StatefulWidget> createState() {
    return _HomeState();
  }
}

class _HomeState extends State<Home> {
  @override
  Widget build(BuildContext context) {
    return IntrinsicWidth(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Container(
            padding: EdgeInsets.all(20),
            child: Text('Nanoshell Examples'),
            color: Colors.blueGrey.shade800,
          ),
          Container(
            padding: EdgeInsets.all(20),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                TextButton(
                    onPressed: () async {
                      final res = await Veil.show(context, () async {
                        final win = await Window.create(
                            ModalWindowBuilder.toInitData());
                        return await win.showModal();
                      });
                      setState(() {
                        _modalWindowResult = res;
                      });
                    },
                    child: Text('Show Modal')),
                if (_modalWindowResult != null)
                  Text('  Result: $_modalWindowResult')
              ],
            ),
          ),
          Padding(
            padding: EdgeInsets.all(20).copyWith(top: 0),
            child: TextButton(
              onPressed: () async {
                if (_dragDropWindow != null) {
                  await _dragDropWindow!.close();
                  _dragDropWindow = null;
                } else {
                  _dragDropWindow =
                      await Window.create(DragDropWindow.toInitData());
                  _dragDropWindow!.closeEvent.addListener(() async {
                    _dragDropWindow = null;
                    setState(() {});
                  });
                }
                setState(() {});
              },
              child: _dragDropWindow == null
                  ? Text('Show Drag & Drop Example')
                  : Text('Hide Drag & Drop Example'),
            ),
          ),
          Padding(
            padding: const EdgeInsets.all(20.0).copyWith(top: 0),
            child: PopupMenu(),
          ),
          Padding(
            padding: const EdgeInsets.all(20.0).copyWith(top: 0),
            child: TextButton(
              onPressed: () async {
                final request =
                    FileOpenRequest(parentWindow: Window.of(context).handle);
                final file = await showFileOpenDialog(request);
                setState(() {
                  if (file != null) {
                    final name = basename(file);
                    fileDialogResult = 'Chosen: $name';
                  } else {
                    fileDialogResult = null;
                  }
                });
              },
              child: Text('Open file dialog'),
            ),
          ),
          AnimatedVisibility(
            visible: fileDialogResult != null,
            duration: Duration(milliseconds: 200),
            alignment: Alignment.center,
            child: Padding(
                padding: const EdgeInsets.all(20.0).copyWith(top: 0),
                child: Center(child: Text(fileDialogResult ?? ''))),
            direction: Axis.vertical,
          ),
        ],
      ),
    );
  }

  dynamic _modalWindowResult;

  String? fileDialogResult;

  Window? _dragDropWindow;
}
