import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:nanoshell/nanoshell.dart';

import 'drag_drop.dart';
import 'modal.dart';
import 'home.dart';
import 'veil.dart';

void main() async {
  runApp(MyApp());
}

class MyApp extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Veil(
        child: DefaultTextStyle(
          style: TextStyle(
            color: Colors.white,
            fontSize: 14,
          ),
          child: Container(
            color: Color.fromARGB(255, 30, 30, 35),
            child: WindowWidget(
              builder: (initData) {
                WindowBuilder? builder;
                builder ??= ModalWindowBuilder.fromInitData(initData);
                builder ??= DragDropWindow.fromInitData(initData);
                builder ??= HomeWindow();
                return builder;
              },
            ),
          ),
        ),
      ),
    );
  }
}
