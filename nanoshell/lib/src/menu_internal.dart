import 'dart:async';

import 'package:flutter/services.dart';
import 'package:nanoshell/src/util.dart';

import 'constants.dart';
import 'menu.dart';

class MenuElement {
  MenuElement({
    required this.id,
    required this.item,
  });

  final int id;

  final MenuItem item;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is MenuElement && other.id == id);

  @override
  int get hashCode => id.hashCode;

  Map serialize() => {
        'id': id,
        'title': item.title,
        'submenu': item.submenu?.currentHandle?.value,
        'enabled': item.action != null || item.submenu != null,
        'separator': item.separator,
        'checked': item.checked,
        'role': item.role != null ? enumToString(item.role) : null,
      };
}

abstract class MenuMaterializer {
  FutureOr<MenuHandle> createOrUpdateMenu(
      Menu menu, List<MenuElement> elements);

  Future<void> destroyMenu(MenuHandle menu);

  MenuMaterializer? createChildMaterializer();
}

class DefaultMaterializer extends MenuMaterializer {
  @override
  FutureOr<MenuHandle> createOrUpdateMenu(
      Menu menu, List<MenuElement> elements) async {
    final serialized = {
      'title': menu.title,
      'role': menu.role != null ? enumToString(menu.role!) : null,
      'items': elements.map((e) => e.serialize()).toList(),
    };

    final handle = menu.currentHandle;

    final res = MenuHandle(
        await MenuManager.instance()._invoke(Methods.menuCreateOrUpdate, {
      'handle': handle?.value,
      'menu': serialized,
    }));
    if (handle != null && handle != res) {
      MenuManager.instance()._activeMenus.remove(handle);
    }
    MenuManager.instance()._activeMenus[res] = menu;
    return res;
  }

  @override
  Future<void> destroyMenu(MenuHandle menuHandle) async {
    await MenuManager.instance()._invoke(Methods.menuDestroy, {
      'handle': menuHandle.value,
    });
    MenuManager.instance()._activeMenus.remove(menuHandle);
  }

  @override
  MenuMaterializer createChildMaterializer() {
    return DefaultMaterializer();
  }
}

final _menuChannel = MethodChannel(Channels.menuManager);

abstract class MenuManagerDelegate {
  void moveToPreviousMenu();
  void moveToNextMenu();
}

class MenuManager {
  static MenuManager instance() => _instance;

  static final _instance = MenuManager();

  MenuManager() {
    _menuChannel.setMethodCallHandler(_onMethodCall);
  }

  Future<dynamic> _invoke(String method, dynamic arg) {
    return _menuChannel.invokeMethod(method, arg);
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == Methods.menuOnAction) {
      final handle = MenuHandle(call.arguments['handle'] as int);
      final id = call.arguments['id'] as int;
      final menu = _activeMenus[handle];
      if (menu != null) {
        menu.onAction(id);
      }
    } else if (call.method == Methods.menubarMoveToPreviousMenu) {
      for (final d in _delegates) {
        d.moveToPreviousMenu();
      }
    } else if (call.method == Methods.menubarMoveToNextMenu) {
      for (final d in _delegates) {
        d.moveToNextMenu();
      }
    }
  }

  Future<void> setAppMenu(MenuHandle handle) async {
    return _menuChannel.invokeMethod(Methods.menuSetAppMenu, {
      'handle': handle.value,
    });
  }

  void registerDelegate(MenuManagerDelegate delegate) {
    _delegates.add(delegate);
  }

  void unregisterDelegate(MenuManagerDelegate delegate) {
    _delegates.remove(delegate);
  }

  void didTransferMenu(Menu menu) {
    if (menu.currentHandle != null) {
      _activeMenus[menu.currentHandle!] = menu;
    }
  }

  final _activeMenus = <MenuHandle, Menu>{};
  final _delegates = <MenuManagerDelegate>[];
}
