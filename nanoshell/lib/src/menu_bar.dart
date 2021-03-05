import 'dart:async';

import 'package:flutter/cupertino.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:nanoshell/nanoshell.dart';
import 'package:nanoshell/src/key_interceptor.dart';
import 'package:nanoshell/src/menu.dart';

import 'menu_internal.dart';

class MenuBar extends StatefulWidget {
  final Menu menu;

  const MenuBar({Key? key, required this.menu}) : super(key: key);

  @override
  State<StatefulWidget> createState() {
    return _MenuBarState(menu);
  }
}

enum _State {
  inactive, // Menubar has no focus
  focused, // Menubar is focused, but no menu is expanded
  active, // Menubar is focused and menu is expanded
}

class _MenuBarState extends State<MenuBar>
    implements MenuMaterializer, MenuManagerDelegate {
  _MenuBarState(Menu menu) : _elements = <MenuElement>[] {
    updateMenu(menu);
  }

  @override
  void didUpdateWidget(covariant MenuBar oldWidget) {
    super.didUpdateWidget(oldWidget);
    updateMenu(widget.menu);
  }

  @override
  Widget build(BuildContext context) {
    if (_firstBuild) {
      // ...of(context) can't be called in init state
      WindowContext.of(context).registerTapCallback(_onWindowTap);
      _firstBuild = false;
    }

    _keys.clear();

    final widgets = _elements.map((e) {
      final key = GlobalKey();
      _keys[e] = key;

      var itemState = MenuItemState.regular;
      if (_selectedElement == e) {
        itemState = MenuItemState.selected;
      } else if (_hoveredElement == e && _selectedElement == null) {
        itemState = MenuItemState.hovered;
      }
      if (e.item.submenu == null && e.item.action == null) {
        itemState = MenuItemState.disabled;
      }

      return MenuBarItem(
        key: key,
        item: e,
        menuBarState: this,
        itemState: itemState,
        showMnemonics: _showMnemonics,
      );
    }).toList();

    return MouseRegion(
      onExit: (e) {
        _onMouseExit();
      },
      child: Listener(
        onPointerDown: _onPointerDown,
        onPointerUp: _onPointerUp,
        onPointerHover: _onHover,
        onPointerMove: _onHover,
        child: Wrap(
          crossAxisAlignment: WrapCrossAlignment.start,
          children: widgets,
        ),
      ),
    );
  }

  void unfocus() {
    _showMnemonics = false;
    _selectedElement = null;
    _state = _State.inactive;
    if (mounted) {
      setState(() {});
    }
  }

  void focus({bool active = true}) {
    setState(() {
      _state = active ? _State.active : _State.focused;
    });
  }

  void _onPointerDown(PointerEvent event) {
    if (event.buttons != 1) {
      return;
    }
    final e = _elementForEvent(event);
    if (e != null) {
      focus();
      selectItem(e);
    }
  }

  void _onPointerUp(PointerEvent event) {
    if (_state == _State.active &&
        _selectedElement != null &&
        _selectedElement!.item.submenu == null) {
      final e = _elementForEvent(event);
      if (e != null && e == _selectedElement) {
        if (e.item.action != null) {
          e.item.action!();
        }
      }
      unfocus();
    }
  }

  void _onWindowTap(PointerEvent event) {
    if (_state != _State.inactive) {
      final e = _elementForEvent(event);
      if (e == null) {
        unfocus();
      }
    }
  }

  void _onHover(PointerEvent event) {
    if (event.localPosition == Offset.zero) {
      // FIXME(knopp) - This seems to be a bug in windows embedder? Investigate
      return;
    }
    final e = _elementForEvent(event);
    if (e != null && !e.item.disabled) {
      onItemHovered(e);
    }
  }

  MenuElement? _elementForEvent(PointerEvent event) {
    for (final e in _keys.entries) {
      final ro2 = e.value.currentContext!.findRenderObject()! as RenderBox;
      final transform = ro2.getTransformTo(null);
      final rect = Rect.fromLTWH(0, 0, ro2.size.width, ro2.size.height);
      final rectTransformed = MatrixUtils.transformRect(transform, rect);
      if (rectTransformed.contains(event.position)) {
        return e.key;
      }
    }
    return null;
  }

  void _onMouseExit() {
    setState(() {
      _hoveredElement = null;
    });
  }

  @override
  MenuMaterializer? createChildMaterializer() {
    // we'll materialize individual menus instead
    return null;
  }

  @override
  FutureOr<MenuHandle> createOrUpdateMenu(
      Menu menu, List<MenuElement> elements) async {
    _elements = elements;
    if (!_elements.contains(_selectedElement)) {
      _selectedElement = null;
    }
    if (!_elements.contains(_hoveredElement)) {
      _hoveredElement = null;
    }
    if (_selectedElement == null) {
      unfocus();
    }
    if (mounted) {
      setState(() {});
    }
    return MenuHandle(0);
  }

  @override
  Future<void> destroyMenu(MenuHandle menu) async {
    setState(() {
      _elements = <MenuElement>[];
    });
  }

  static Completer? _currentMenuCompleter;

  void updateMenu(Menu menu) {
    final previousCompleter = _currentMenuCompleter;

    final menuCompleter = Completer();
    _currentMenuCompleter = menuCompleter;

    menu.materialize((handle) async {
      return menuCompleter.future;
    });

    if (previousCompleter != null) {
      previousCompleter.complete();
    }
  }

  void selectItem(MenuElement item, {bool withKeyboard = false}) async {
    if (_menuVisible == item) {
      unfocus();
      return;
    }

    if (_menuVisible != null &&
        _selectedElement?.item.submenu != null &&
        _selectedElement?.item.submenu!.currentHandle != null &&
        item.item.submenu == null) {
      await Window.of(context)
          .hidePopupMenu(_selectedElement!.item.submenu!.currentHandle!);
    }

    setState(() {
      _selectedElement = item;
    });

    final cookie = ++_cookie;
    if (item.item.submenu != null && _state == _State.active) {
      await _displayMenu(item, withKeyboard, cookie);
    } else {
      _menuVisible = null;
    }
  }

  Future<void> _displayMenu(
      MenuElement item, bool withKeyboard, int cookie) async {
    if (item.item.submenu == null) {
      return;
    }
    final submenu = item.item.submenu!;

    _menuVisible = item;

    final win = Window.of(context);
    final box = _keys[item]!.currentContext!.findRenderObject() as RenderBox;
    final itemRect = Rect.fromLTWH(0, 0, box.size.width, box.size.height);
    final transform = box.getTransformTo(null);
    final transformed = MatrixUtils.transformRect(transform, itemRect);

    final menubarObject = context.findRenderObject() as RenderBox;
    final menubarRect = Rect.fromLTWH(
        0, 0, menubarObject.size.width, menubarObject.size.height);
    final trackingRect = MatrixUtils.transformRect(
        menubarObject.getTransformTo(null), menubarRect);

    final res = await win.showPopupMenu(
      submenu,
      transformed.bottomLeft,
      trackingRect: trackingRect,
      itemRect: transformed,
      preselectFirst: withKeyboard,
    );

    if (res.itemSelected) {
      unfocus();
    }
    await Future.delayed(Duration(milliseconds: 100));
    if (_cookie == cookie) {
      setState(() {
        _menuVisible = null;
        if (_state == _State.active) {
          _state = _State.focused;
        }
      });
    }
  }

  void onItemHovered(MenuElement item) {
    if (_hoveredElement != item) {
      setState(() {
        _hoveredElement = item;
      });
    }

    if (_selectedElement == item) {
      return;
    } else if (_state == _State.active) {
      selectItem(item);
    } else if (_state == _State.focused) {
      setState(() {
        _selectedElement = item;
      });
    }
  }

  bool get _hasEnabledElements {
    return _elements.any((element) => !element.item.disabled);
  }

  bool _onRawKeyEvent(RawKeyEvent event) {
    final hasEnabledElements = _hasEnabledElements;
    var focusRequested = false;

    if (event is RawKeyDownEvent &&
        event.logicalKey == LogicalKeyboardKey.altLeft) {
      _ignoreNextAltKeyUp = false;
      if (hasEnabledElements) {
        setState(() {
          _missedMnemonics = false;
          _showMnemonics = true;
        });
      }
      return false;
    }
    if (event is RawKeyUpEvent) {
      if (event.logicalKey == LogicalKeyboardKey.altLeft && _showMnemonics) {
        if (_state != _State.inactive || _missedMnemonics) {
          if (!_ignoreNextAltKeyUp) {
            unfocus();
          }
          _ignoreNextAltKeyUp = false;
        } else if (!_missedMnemonics && hasEnabledElements) {
          focus(active: false);
          focusRequested = true;
          setState(() {
            _selectedElement = _elements[0];
          });
        }
      }
    }
    if (event is RawKeyDownEvent &&
        event.character != null &&
        (_showMnemonics || _state != _State.inactive)) {
      for (final e in _elements) {
        final mnemonics = Mnemonics.parse(e.item.title);
        if (mnemonics.character != null &&
            mnemonics.character!.toLowerCase() ==
                event.character!.toLowerCase()) {
          if (e.item.submenu != null) {
            _showMnemonics = true;
            _missedMnemonics = false;
            _ignoreNextAltKeyUp = true;
            focus();
            selectItem(e, withKeyboard: true);
          } else if (e.item.action != null) {
            e.item.action!();
            if (_state == _State.focused) {
              unfocus();
            }
          }
          return true;
        }
      }
      if (event.character == ' ') {
        Window.of(context).showSystemMenu();
        unfocus();
        return true;
      }
    }

    if (event is RawKeyDownEvent && _state != _State.inactive) {
      if (event.logicalKey == LogicalKeyboardKey.escape &&
          (_menuVisible == null || _selectedElement?.item.submenu == null)) {
        unfocus();
      } else if (event.logicalKey == LogicalKeyboardKey.arrowLeft) {
        _moveToMenu(-1);
        return true;
      } else if (event.logicalKey == LogicalKeyboardKey.arrowRight) {
        _moveToMenu(1);
        return true;
      } else if (_selectedElement!.item.submenu != null &&
          (event.logicalKey == LogicalKeyboardKey.arrowDown ||
              event.logicalKey == LogicalKeyboardKey.arrowUp ||
              event.logicalKey == LogicalKeyboardKey.enter)) {
        if (_menuVisible == null && _selectedElement != null) {
          focus();
          selectItem(_selectedElement!, withKeyboard: true);
          return true;
        }
      } else if (_selectedElement!.item.submenu == null &&
          event.logicalKey == LogicalKeyboardKey.enter) {
        if (_selectedElement!.item.action != null) {
          _selectedElement!.item.action!();
          unfocus();
        }
      }
    }

    // hasFocus may be false for a bit after requesting focus
    if (_showMnemonics && !focusRequested) {
      _missedMnemonics = true;
    }

    if (_state != _State.inactive) {
      return true;
    } else {
      return false;
    }
  }

  @override
  void initState() {
    super.initState();
    MenuManager.instance().registerDelegate(this);
    KeyInterceptor.instance.registerHandler(_onRawKeyEvent);
    _firstBuild = true;
  }

  @override
  void dispose() {
    super.dispose();
    MenuManager.instance().unregisterDelegate(this);
    KeyInterceptor.instance.unregisterHandler(_onRawKeyEvent);
    WindowContext.of(context).unregisterTapCallback(_onWindowTap);
  }

  void _moveToMenu(int delta) {
    if (_selectedElement != null) {
      final index = _elements.indexOf(_selectedElement!);
      if (index != -1) {
        var nextIndex = index;
        for (var i = 0; i < _elements.length; ++i) {
          nextIndex = nextIndex + delta;
          if (nextIndex == _elements.length) {
            nextIndex = 0;
          } else if (nextIndex < 0) {
            nextIndex = _elements.length - 1;
          }
          if (!_elements[nextIndex].item.disabled) {
            break;
          }
        }
        if (nextIndex != index) {
          selectItem(_elements[nextIndex], withKeyboard: true);
        }
      }
    }
  }

  @override
  void moveToNextMenu() {
    _moveToMenu(1);
  }

  @override
  void moveToPreviousMenu() {
    _moveToMenu(-1);
  }

  var _state = _State.inactive;

  int _cookie = 0;

  List<MenuElement> _elements;

  // item currently hovered; this is mostly used to keep track of mouse hover
  // and used to restore selected item after losing focus
  MenuElement? _hoveredElement;

  // if unfocused, hovered item; if focused, either hovered or selected by keyboard,
  // depending on what event was latest
  MenuElement? _selectedElement;

  // currently expanded menu
  MenuElement? _menuVisible;

  // Used to retrieve render objects
  final _keys = <MenuElement, GlobalKey>{};

  // whether mnemonics are visible
  bool _showMnemonics = false;

  // key pressed while mnemonics is visible that didn't trigger a menu;
  // when this is true, releasing alt will not focus menubar
  bool _missedMnemonics = false;

  // whether build() is called for the first time
  bool _firstBuild = true;

  // Under normal circumstances, releasing ALT key unfocuses menu; However
  // this is not true when mnemonics key was pressed
  bool _ignoreNextAltKeyUp = false;
}

enum MenuItemState {
  regular,
  hovered,
  selected,
  disabled,
}

class Mnemonics {
  Mnemonics(this.text, this.mnemonicIndex);

  static Mnemonics parse(String s) {
    var index = -1;
    var mnemonic = false;
    final text = StringBuffer();
    for (final c in s.characters) {
      if (c == '&') {
        if (!mnemonic) {
          mnemonic = true;
          continue;
        } else {
          text.write('&');
          mnemonic = false;
          continue;
        }
      }
      if (mnemonic) {
        index = text.length;
        mnemonic = false;
      }
      text.write(c);
    }
    return Mnemonics(text.toString(), index);
  }

  String? get character {
    return mnemonicIndex != -1 ? text[mnemonicIndex] : null;
  }

  TextSpan asTextSpan(TextStyle baseStyle, [bool showMnemonics = true]) {
    final index = showMnemonics ? mnemonicIndex : -1;
    return TextSpan(children: [
      if (index > 0)
        TextSpan(
          text: text.substring(0, index),
          style: baseStyle,
        ),
      if (index != -1)
        TextSpan(
            text: text[index],
            style: baseStyle.copyWith(decoration: TextDecoration.underline)),
      if (index < text.length - 1)
        TextSpan(
          text: text.substring(index + 1),
          style: baseStyle,
        ),
    ]);
  }

  final String text;
  final int mnemonicIndex;
}

class MenuBarItem extends StatelessWidget {
  final MenuElement item;
  final _MenuBarState menuBarState;
  final MenuItemState itemState;
  final bool showMnemonics;

  const MenuBarItem({
    Key? key,
    required this.item,
    required this.menuBarState,
    required this.itemState,
    required this.showMnemonics,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Builder(builder: (context) {
      final mnemonic = Mnemonics.parse(item.item.title);
      Color background;
      Color foreground;
      switch (itemState) {
        case MenuItemState.regular:
          background = Colors.transparent;
          foreground = Colors.white;
          break;
        case MenuItemState.hovered:
          background = Colors.white.withAlpha(50);
          foreground = Colors.white;
          break;
        case MenuItemState.selected:
          background = Colors.white.withAlpha(100);
          foreground = Colors.white;
          break;
        case MenuItemState.disabled:
          background = Colors.transparent;
          foreground = Colors.white.withAlpha(100);
          break;
      }
      return Container(
        padding: EdgeInsets.symmetric(horizontal: 10, vertical: 5),
        color: background,
        child: RichText(
          text: mnemonic.asTextSpan(
              DefaultTextStyle.of(context).style.copyWith(color: foreground),
              showMnemonics),
        ),
      );
    });
  }
}
