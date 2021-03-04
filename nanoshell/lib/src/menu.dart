import 'dart:collection';
import 'package:flutter/material.dart';

import 'menu_internal.dart';
import 'mutex.dart';

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

  bool get disabled => submenu == null && _action == null;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (separator && other is MenuItem && other.separator) ||
      (other is MenuItem && title == other.title && submenu == other.submenu);

  @override
  int get hashCode => hashValues(title, separator, submenu);
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

  Future<MenuHandle> materialize([MenuMaterializer? materializer]) async {
    _materializer = materializer;
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
      _materializer ??= DefaultMaterializer();

      final childMaterializer = _materializer!.createChildMaterializer();
      if (childMaterializer != null) {
        for (final element in _currentElements) {
          if (element.item.submenu != null) {
            await element.item.submenu!
                ._materializeSubmenu(this, childMaterializer);
          }
        }
      }

      _currentHandle =
          await _materializer!.createOrUpdateMenu(this, _currentElements);
      return _currentHandle!;
    }
  }

  Future<void> _materializeSubmenu(
      Menu parent, MenuMaterializer materializer) async {
    assert(_materializeParent == null || identical(_materializeParent, parent),
        'Menu can not be moved to another parent while materialized');
    _materializeParent = parent;
    _materializer = materializer;
    await _materializeLocked();
  }

  Menu? _materializeParent;
  MenuMaterializer? _materializer;

  Future<void> _unmaterializeLocked() async {
    if (_currentHandle != null && _materializer != null) {
      for (final element in _currentElements) {
        if (element.item.submenu?._materializeParent == this) {
          await element.item.submenu!._unmaterializeLocked();
        }
      }
      _materializeParent = null;
      await _materializer!.destroyMenu(_currentHandle!);
      _materializer = null;
      _currentHandle = null;
    }
    _pastActions.clear();
  }

  Future<void> _updateLocked() async {
    final removed = <Menu>[];
    final preserved = <Menu>[];
    final added = <Menu>[];

    if (_materializer == null) {
      return;
    }

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
    if (_currentHandle != null && _materializer != null) {
      _currentHandle =
          await _materializer!.createOrUpdateMenu(this, _currentElements);
    }
  }

  bool _onAction(int itemId) {
    for (final e in _currentElements) {
      if (e.id == itemId && e.item.action != null) {
        e.item.action!();
        return true;
      }
    }
    for (final e in _currentElements) {
      if (e.item.submenu != null && e.item.submenu!._onAction(itemId)) {
        return true;
      }
    }
    return false;
  }

  void onAction(int itemId) {
    if (_onAction(itemId)) {
      return;
    }
    final pastAction = _pastActions[itemId];
    if (pastAction != null) {
      pastAction();
    }
  }

  List<MenuElement> _currentElements = [];

  // temporarily save action for removed item; this is to ensure that
  // when item is removed right after user selects it, we can still deliver the
  // callback
  final _pastActions = <int, VoidCallback>{};

  MenuHandle? get currentHandle => _currentHandle;

  MenuHandle? _currentHandle;

  List<MenuElement> _mergeElements(List<MenuItem> items, List<Menu> outRemoved,
      List<Menu> outPreserved, List<Menu> outAdded) {
    final res = <MenuElement>[];

    _pastActions.clear();

    final current =
        HashMap.fromEntries(_currentElements.map((e) => MapEntry(e.item, e)));

    // Preserve separators in the order they came; This is useful for cocoa which
    // can not convert existing item to separator
    final currentSeparators =
        _currentElements.where((element) => element.item.separator).toList();

    for (final i in items) {
      MenuElement? existing;
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
        res.add(MenuElement(id: _nextItemId++, item: i));
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
}

int _nextItemId = 1;

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
