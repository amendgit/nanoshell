import 'dart:collection';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'mutex.dart';
import 'constants.dart';

class MenuItem {
  MenuItem({
    required this.title,
    required VoidCallback? action,
    this.checked = false,
  })  : separator = false,
        _action = action,
        submenu = null;

  MenuItem.menu({
    required this.title,
    required this.submenu,
  })   : separator = false,
        _action = null,
        checked = false;

  MenuItem.children({
    required String title,
    required List<MenuItem> children,
  }) : this.builder(title: title, builder: () => children);

  MenuItem.builder({
    required this.title,
    required MenuBuilder builder,
  })   : _action = null,
        separator = false,
        checked = false,
        submenu = Menu._(builder, ephemeral: true);

  MenuItem.separator()
      : title = '',
        _action = null,
        separator = true,
        checked = false,
        submenu = null;

  final String title;

  VoidCallback? get action => _action;
  VoidCallback? _action;

  void _replaceAction(VoidCallback? action) {
    _action = action;
  }

  final Menu? submenu;

  final bool separator;
  final bool checked;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (separator && other is MenuItem && other.separator) ||
      (other is MenuItem && title == other.title && submenu == other.submenu);

  @override
  int get hashCode => hashValues(title, action, submenu);
}

typedef MenuBuilder = List<MenuItem> Function();

class Menu {
  Menu(MenuBuilder builder) : this._(builder, ephemeral: false);

  Menu._(this.builder, {required bool ephemeral}) : _ephemeral = ephemeral {
    final removed = <Menu>[];
    final preserved = <Menu>[];
    final added = <Menu>[];
    _currentElements = _mergeElements(builder(), removed, preserved, added);
  }

  static final _mutex = Mutex();

  final bool _ephemeral;
  MenuBuilder builder;

  Future<MenuHandle> materialize() async {
    return _mutex.protect(() async {
      return _materializeLocked();
    });
  }

  Future<void> unmaterialize() async {
    await _mutex.protect(() => _unmaterializeLocked());
  }

  Future<void> update() async {
    return _mutex.protect(() => _updateLocked());
  }

  Future<MenuHandle> _materializeLocked() async {
    if (_currentHandle != null) {
      return _currentHandle!;
    } else {
      for (final element in _currentElements) {
        if (element.item.submenu != null) {
          await element.item.submenu!._materializeSubmenu(this);
        }
      }

      return _updatePlatformMenu();
    }
  }

  Future<void> _materializeSubmenu(Menu parent) async {
    assert(_materializeParent == null || identical(_materializeParent, parent),
        'Menu can not be moved to another parent while materialized');
    _materializeParent = parent;
    await _materializeLocked();
  }

  Menu? _materializeParent;

  Future<MenuHandle> _updatePlatformMenu() async {
    final serialized = {
      'items': _currentElements.map((e) => e.serialize()).toList()
    };

    final res = MenuHandle(
        await _MenuManager.instance().invoke(Methods.menuCreateOrUpdate, {
      'handle': _currentHandle?.value,
      'menu': serialized,
    }));
    if (_currentHandle != null && _currentHandle != res) {
      _MenuManager.instance()._activeMenus.remove(_currentHandle);
    }
    _currentHandle = res;
    _MenuManager.instance()._activeMenus[res] = this;
    return res;
  }

  Future<void> _unmaterializeLocked() async {
    if (_currentHandle != null) {
      for (final element in _currentElements) {
        if (element.item.submenu != null) {
          await element.item.submenu!._unmaterializeLocked();
        }
      }
      _materializeParent = null;
      await _MenuManager.instance().invoke(Methods.menuDestroy, {
        'handle': _currentHandle?.value,
      });
      _MenuManager.instance()._activeMenus.remove(_currentHandle!);
      _currentHandle = null;
    }
    _pastActions.clear();
  }

  Future<void> _updateLocked() async {
    final removed = <Menu>[];
    final preserved = <Menu>[];
    final added = <Menu>[];
    _currentElements = _mergeElements(builder(), removed, preserved, added);
    for (final menu in preserved) {
      await menu._updateLocked();
    }
    for (final menu in removed) {
      await menu._unmaterializeLocked();
    }
    if (_currentHandle != null) {
      for (final menu in added) {
        if (_currentHandle != null) {
          await menu._materializeLocked();
        }
      }
    }
    if (_currentHandle != null) {
      await _updatePlatformMenu();
    }
  }

  void _onAction(int itemId) {
    for (final e in _currentElements) {
      if (e.id == itemId && e.item.action != null) {
        e.item.action!();
        return;
      }
    }
    final pastAction = _pastActions[itemId];
    if (pastAction != null) {
      pastAction();
    }
  }

  List<_MenuElement> _currentElements = [];

  // temporarily save action for removed item; this is to ensure that
  // when item is removed right after user selects it, we can still deliver the
  // callback
  final _pastActions = <int, VoidCallback>{};

  MenuHandle? _currentHandle;

  List<_MenuElement> _mergeElements(List<MenuItem> items, List<Menu> outRemoved,
      List<Menu> outPreserved, List<Menu> outAdded) {
    final res = <_MenuElement>[];

    _pastActions.clear();

    final current =
        HashMap.fromEntries(_currentElements.map((e) => MapEntry(e.item, e)));

    // Preserve separators in the order they came; This is useful for cocoa which
    // can not convert existing item to separator
    final currentSeparators =
        _currentElements.where((element) => element.item.separator).toList();

    for (final i in items) {
      _MenuElement? existing;
      if (i.separator) {
        if (currentSeparators.isNotEmpty) {
          existing = currentSeparators.removeAt(0);
        }
      } else {
        existing = current[i];
      }
      if (existing != null) {
        res.add(existing);
        current.remove(i);

        // action is not part of equality check, so if action changed we preserve old
        // item but update the action
        existing.item._replaceAction(i.action);

        if (existing.item.submenu != null) {
          if (existing.item.submenu!._ephemeral) {
            // For ephemeral submenus we preserve the submenu instance but update builder
            assert(i.submenu!._ephemeral);
            existing.item.submenu!._updateFrom(i.submenu!);
          }
          outPreserved.add(existing.item.submenu!);
        }
      } else {
        if (i.submenu != null) {
          outAdded.add(i.submenu!);
        }
        res.add(_MenuElement(id: _nextId++, item: i));
      }
    }

    // items not used anymore
    for (final i in current.values) {
      final submenu = i.item.submenu;
      if (submenu != null) {
        outRemoved.add(submenu);
      } else if (i.item.action != null) {
        _pastActions[i.id] = i.item.action!;
      }
    }

    return res;
  }

  void _updateFrom(Menu newMenu) {
    assert(_ephemeral);
    builder = newMenu.builder;
  }

  @override
  int get hashCode {
    // consident with ==
    return _ephemeral.hashCode;
  }

  // Menu is considered equal when same identity or both are ephemeral;
  // This is because when updating ephemeral menu we'll preserve instance
  // and update builder
  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other is Menu && _ephemeral && other._ephemeral);
  }

  int _nextId = 1;
}

class MenuHandle {
  const MenuHandle(this.value);

  final int value;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is MenuHandle && other.value == value);

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => 'MenuHandle($value)';
}

class _MenuElement {
  _MenuElement({
    required this.id,
    required this.item,
  });

  final int id;

  final MenuItem item;

  Map serialize() => {
        'id': id,
        'title': item.title,
        'submenu': item.submenu?._currentHandle?.value,
        'enabled': item.action != null || item.submenu != null,
        'separator': item.separator,
        'checked': item.checked,
      };
}

final _menuChannel = MethodChannel(Channels.menuManager);

class _MenuManager {
  static _MenuManager instance() => _instance;

  static final _instance = _MenuManager();

  _MenuManager() {
    _menuChannel.setMethodCallHandler(_onMethodCall);
  }

  Future<dynamic> invoke(String method, dynamic arg) {
    return _menuChannel.invokeMethod(method, arg);
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == Methods.menuOnAction) {
      final handle = MenuHandle(call.arguments['handle'] as int);
      final id = call.arguments['id'] as int;
      final menu = _activeMenus[handle];
      if (menu != null) {
        menu._onAction(id);
      }
    } else if (call.method == Methods.menubarMoveToPreviousMenu) {
      print('Move to Previous Menu');
    } else if (call.method == Methods.menubarMoveToNextMenu) {
      print('Move to Next Menu');
    }
  }

  final _activeMenus = <MenuHandle, Menu>{};
}
