import 'dart:async';
import 'dart:convert';

import 'package:flutter/cupertino.dart';
import 'package:flutter/services.dart';

const channel = BasicMessageChannel('nanoshell/keyevent', BinaryCodec());

class RawKeyEventEx {
  RawKeyEventEx(
      {required RawKeyEvent event,
      required this.keyWithoutModifiers,
      this.keyWithoutModifiers2})
      : event = event,
        controlPressed = event.isControlPressed,
        altPressed = event.isAltPressed,
        metaPressed = event.isMetaPressed,
        shiftPressed = event.isShiftPressed;

  // Original key event
  final RawKeyEvent event;

  // Key event with "original" key without modifiers
  final LogicalKeyboardKey keyWithoutModifiers;

  // Alternate key without modifiers; This would be with shift applied, but only
  // if shift is presed; This is used to handle accelerators such as shift + } on
  // US keyboard; Note that this will also match shift + ]; There is no way to
  // distinguish these two, so we match either
  final LogicalKeyboardKey? keyWithoutModifiers2;

  final bool controlPressed;
  final bool altPressed;
  final bool metaPressed;
  final bool shiftPressed;
}

typedef KeyInterceptorHandler = bool Function(RawKeyEventEx event);

RawKeyEventEx _keyEventFromMessage(Map<String, dynamic> message) {
  final noModifiers = message['charactersIgnoringModifiersEx'] as String?;
  final noModifiersExceptShift =
      message['charactersIgnoringModifiersExceptShiftEx'] as String?;
  final event = RawKeyEvent.fromMessage(message);

  var noModifiersKey = event.logicalKey;
  var noModifiersExceptShiftKey;

  if (noModifiers != null) {
    noModifiersKey = _keyFromCharacters(noModifiers, event);
  }

  if (noModifiersExceptShift != null && event.isShiftPressed) {
    noModifiersExceptShiftKey =
        _keyFromCharacters(noModifiersExceptShift, event);
  }

  return RawKeyEventEx(
      event: event,
      keyWithoutModifiers: noModifiersKey,
      keyWithoutModifiers2: noModifiersExceptShiftKey);
}

LogicalKeyboardKey _keyFromCharacters(String characters, RawKeyEvent event) {
  final data = event.data;
  if (data is RawKeyEventDataMacOs) {
    final newEvent = RawKeyEventDataMacOs(
      characters: characters,
      charactersIgnoringModifiers: characters,
      keyCode: data.keyCode,
      modifiers: data.modifiers,
    );
    return newEvent.logicalKey;
  } else if (data is RawKeyEventDataWindows) {
    final newEvent = RawKeyEventDataWindows(
      characterCodePoint: characters.codeUnitAt(0),
      keyCode: data.keyCode,
      modifiers: data.modifiers,
      scanCode: data.scanCode,
    );
    return newEvent.logicalKey;
  } else {
    return event.logicalKey;
  }
}

class KeyInterceptor {
  KeyInterceptor._() {
    channel.setMessageHandler(_onMessage);
  }

  static ByteData _dataForHandled(bool handled) {
    final res = <String, dynamic>{'handled': handled};
    return StringCodec().encodeMessage(json.encode(res))!;
  }

  Future<ByteData> _onMessage(ByteData? message) {
    final string = StringCodec().decodeMessage(message) ?? '';
    final keyMessage = json.decode(string);

    final event = _keyEventFromMessage(keyMessage);

    for (final handler in List<KeyInterceptorHandler>.from(_handlers)) {
      if (handler(event)) {
        return Future.value(_dataForHandled(true));
      }
    }

    final completer = Completer<ByteData>();
    WidgetsBinding.instance?.defaultBinaryMessenger
        .handlePlatformMessage('flutter/keyevent', message, (data) {
      // macos with FN pressed seems to return null?
      data ??= _dataForHandled(false);

      completer.complete(data);
    });
    return completer.future;
  }

  void registerHandler(KeyInterceptorHandler handler) {
    _handlers.add(handler);
  }

  void unregisterHandler(KeyInterceptorHandler handler) {
    _handlers.remove(handler);
  }

  final _handlers = <KeyInterceptorHandler>[];

  static final KeyInterceptor instance = KeyInterceptor._();
}
