import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter/src/widgets/framework.dart';
import 'package:nanoshell/nanoshell.dart';

class MenuBarWindow extends WindowBuilder {
  List<MenuItem> buildMenu() => [
        MenuItem.children(title: '&Fist Item', children: [
          MenuItem(title: 'Fist Item', action: null),
          MenuItem(title: 'Second Item', action: () {}),
        ]),
        MenuItem.children(title: 'Second &item', children: [
          MenuItem(title: 'Fist && Item', action: () {}),
          MenuItem(
              title: 'S&econd Item',
              action: () {
                print('SECOND');
              }),
        ]),
        MenuItem(
            title: 'A&ction Item',
            action: () {
              print('Action');
            }),
        MenuItem(title: 'Action Item Disabled', action: null),
        MenuItem.children(title: '&Third item', children: [
          MenuItem(
              title: 'Fist Item',
              action: () {
                print('FIRST!');
              }),
          MenuItem.children(title: 'Second Item', children: [
            MenuItem(
                title: '&Fist Item',
                action: () {
                  print('>> First');
                }),
            MenuItem(
                title: 'Second Item',
                action: () {
                  print('>> Second');
                }),
          ]),
          MenuItem(
              title: 'Third Item',
              action: () {
                print('Third!');
              }),
        ])
      ];

  final focus = FocusNode();

  @override
  Widget build(BuildContext context) {
    final menu = Menu(buildMenu);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          color: Colors.blueGrey,
          child: MenuBar(menu: menu),
        ),
        Expanded(
          child: Center(
            child: Material(
              child: TextField(
                autofocus: true,
                focusNode: focus,
              ),
            ),
          ),
        ),
      ],
    );
  }
}
