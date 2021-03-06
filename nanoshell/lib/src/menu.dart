import 'dart:async';
import 'dart:collection';
import 'package:flutter/material.dart';

import 'accelerator.dart';
import 'menu_internal.dart';
import 'mutex.dart';

enum MenuItemRole {
  minimizeWindow,
  zoomWindow,
  bringAllToFront,
}

enum MenuRole {
  // macOS specific; Menus marked with window will have additional Window specific items in it
  window,
}

class MenuItem {
  MenuItem({
    required this.title,
    required this.action,
    this.checked = false,
    this.accelerator,
  })  : separator = false,
        submenu = null,
        role = null;

  MenuItem.menu({
    required this.title,
    required this.submenu,
  })   : separator = false,
        action = null,
        checked = false,
        role = null,
        accelerator = null;

  MenuItem.children({
    required String title,
    required List<MenuItem> children,
    MenuRole? role,
  }) : this.builder(
          title: title,
          builder: () => children,
          role: role,
        );

  MenuItem.builder({
    required String title,
    required MenuBuilder builder,
    MenuRole? role,
  }) : this.menu(
          title: title,
          submenu: Menu._(builder, title: title, role: role),
        );

  MenuItem.withRole({
    required MenuItemRole role,
    String? title,
    this.accelerator,
  })  : action = null,
        separator = false,
        checked = false,
        title = title ?? _titleForRole(role),
        role = role,
        submenu = null;

  MenuItem.separator()
      : title = '',
        action = null,
        separator = true,
        checked = false,
        role = null,
        submenu = null,
        accelerator = null;

  final String title;
  final MenuItemRole? role;

  final VoidCallback? action;

  final Menu? submenu;

  final bool separator;
  final bool checked;

  bool get disabled => submenu == null && action == null;

  final Accelerator? accelerator;

  bool _canBeUpdatedFrom(MenuItem other) =>
      identical(this, other) ||
      (separator && other.separator) ||
      (title == other.title &&
          (submenu == null) == (other.submenu == null) &&
          role == other.role);

  int get _canBeUpdatedFromHashCode =>
      hashValues(title, separator, submenu != null);

  static String _titleForRole(MenuItemRole role) {
    switch (role) {
      case MenuItemRole.minimizeWindow:
        return 'Minimize';
      case MenuItemRole.zoomWindow:
        return 'Zoom';
      case MenuItemRole.bringAllToFront:
        return 'Bring All to Front';
    }
  }
}

typedef MenuBuilder = List<MenuItem> Function();

class Menu {
  Menu(
    MenuBuilder builder, {
    String title = '',
    MenuRole? role,
  }) : this._(
          builder,
          title: title,
          role: role,
        );

  Menu._(
    this.builder, {
    required this.title,
    this.role,
  });

  final MenuRole? role;
  final String title;
  MenuBuilder builder;

  static final mutex = Mutex();

  Future<T> materialize<T>(Future<T> Function(MenuHandle) callback,
      [MenuMaterializer? materializer]) async {
    final handle = await mutex.protect(() async {
      _materializer = materializer;
      return _materializeLocked();
    });
    final res = await callback(handle);
    await mutex.protect(() => _unmaterializeLocked());
    return res;
  }

  Future<void> update() async {
    final res = mutex.protect(() => _updateLocked());
    return res;
  }

  // fired when replacing app menu; used to release handle in materialize
  static Completer? _currentAppMenuCompleter;

  // macOS specific. Sets this menu as application menu. It will be shown
  // for every window that doesn't have window specific menu.
  Future<void> setAsAppMenu() {
    final previousCompleter = _currentAppMenuCompleter;

    final functionCompleter = Completer();
    final menuCompleter = Completer();
    _currentAppMenuCompleter = menuCompleter;

    materialize((handle) async {
      await MenuManager.instance().setAppMenu(handle);
      functionCompleter.complete();
      // keep the handle alive until completer
      return menuCompleter.future;
    });

    if (previousCompleter != null) {
      previousCompleter.complete();
    }

    return functionCompleter.future;
  }

  Future<MenuHandle> _materializeLocked() async {
    if (_currentHandle != null) {
      return _currentHandle!;
    } else {
      final removed = <Menu>[];
      final preserved = <Menu>[];
      final added = <Menu>[];

      _currentElements = _mergeElements(builder(), removed, preserved, added);

      _materializer ??= DefaultMaterializer();

      final handle =
          await _materializer!.createOrUpdateMenuPre(this, _currentElements);

      final childMaterializer = _materializer!.createChildMaterializer();
      if (childMaterializer != null) {
        for (final element in _currentElements) {
          if (element.item.submenu != null) {
            await element.item.submenu!
                ._materializeSubmenu(this, childMaterializer);
          }
        }
      }

      _currentHandle = await _materializer!
          .createOrUpdateMenuPost(this, _currentElements, handle);
      return _currentHandle!;
    }
  }

  Future<void> _materializeSubmenu(
      Menu parent, MenuMaterializer materializer) async {
    assert(
        _materializeParent == null ||
            identical(_materializeParent!._transferTarget, parent),
        'Menu can not be moved to another parent while materialized');
    _materializeParent = parent;
    _materializer = materializer;
    await _materializeLocked();
  }

  Menu? _materializeParent;
  MenuMaterializer? _materializer;

  Future<void> _unmaterializeLocked() {
    return _transferTarget._doUnmaterialize();
  }

  Future<void> _doUnmaterialize() async {
    assert(_currentHandle != null && _materializer != null);
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
    _currentElements.clear();
  }

  Future<void> _updateLocked() async {
    final removed = <Menu>[];
    final preserved = <Menu>[];
    final added = <Menu>[];

    if (_materializer == null) {
      return;
    }

    _currentElements = _mergeElements(builder(), removed, preserved, added);

    final handle =
        await _materializer!.createOrUpdateMenuPre(this, _currentElements);

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
      _currentHandle = await _materializer!
          .createOrUpdateMenuPost(this, _currentElements, handle);
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
      if (e.item.submenu != null && e.item.submenu!.onAction(itemId)) {
        return true;
      }
    }
    return false;
  }

  bool onAction(int itemId) {
    if (_onAction(itemId)) {
      return true;
    }
    final pastAction = _pastActions[itemId];
    if (pastAction != null) {
      pastAction();
      return true;
    }
    return false;
  }

  List<MenuElement> _currentElements = [];

  // temporarily save action for removed item; this is to ensure that
  // when item is removed right after user selects it, we can still deliver the
  // callback
  final _pastActions = <int, VoidCallback>{};

  MenuHandle? get currentHandle => _transferTarget._currentHandle;

  MenuHandle? _currentHandle;

  List<MenuElement> _mergeElements(List<MenuItem> items, List<Menu> outRemoved,
      List<Menu> outPreserved, List<Menu> outAdded) {
    final res = <MenuElement>[];

    _pastActions.clear();

    final currentByItem = HashMap<MenuItem, MenuElement>(
      equals: (e1, e2) => e1._canBeUpdatedFrom(e2),
      hashCode: (e) => e._canBeUpdatedFromHashCode,
    );
    currentByItem.addEntries(_currentElements.map((e) => MapEntry(e.item, e)));

    final currentByMenu = HashMap.fromEntries(_currentElements
        .where((element) => element.item.submenu != null)
        .map((e) => MapEntry(e.item.submenu!, e)));

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
      } else if (_currentElements.isNotEmpty) {
        // if there is item with this exact submenu, use it
        existing = currentByMenu.remove(i.submenu);

        // otherwise take item with same name but possible different submenu,
        // as long as new item has not bee nmaterialized
        if (i.submenu?.currentHandle == null) {
          existing ??= currentByItem.remove(i);
        }
      }
      if (existing != null) {
        res.add(MenuElement(id: existing.id, item: i));

        if (existing.item.submenu != null &&
            !identical(existing.item.submenu, i.submenu)) {
          i.submenu!._transferFrom(existing.item.submenu!);
          outPreserved.add(i.submenu!);
        }
      } else {
        res.add(MenuElement(id: _nextItemId++, item: i));
        if (i.submenu != null) {
          outAdded.add(i.submenu!);
        }
      }
    }

    // items not used anymore
    for (final i in currentByItem.values) {
      final submenu = i.item.submenu;
      if (submenu != null) {
        outRemoved.add(submenu);
      } else if (i.item.action != null) {
        _pastActions[i.id] = i.item.action!;
      }
    }

    return res;
  }

  Menu? _transferedTo;

  Menu get _transferTarget {
    return _transferedTo != null ? _transferedTo!._transferTarget : this;
  }

  void _transferFrom(Menu oldMenu) {
    assert(_currentHandle == null);
    assert(_currentElements.isEmpty);
    _currentHandle = oldMenu._currentHandle;
    oldMenu._currentHandle = null;

    _currentElements = oldMenu._currentElements;
    oldMenu._currentElements = <MenuElement>[];

    _materializeParent = oldMenu._materializeParent;
    oldMenu._materializeParent = null;

    _materializer = oldMenu._materializer;
    oldMenu._materializer = null;

    _pastActions.addEntries(oldMenu._pastActions.entries);
    oldMenu._pastActions.clear();

    oldMenu._transferedTo = this;

    MenuManager.instance().didTransferMenu(this);
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
