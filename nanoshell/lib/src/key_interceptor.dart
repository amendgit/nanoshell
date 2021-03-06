import 'dart:async';
import 'dart:convert';

import 'package:flutter/cupertino.dart';
import 'package:flutter/services.dart';

const channel = BasicMessageChannel('nanoshell/keyevent', BinaryCodec());

class RawKeyEventEx {
  RawKeyEventEx({required RawKeyEvent event, required this.keyWithoutModifiers})
      : event = event,
        controlPressed = event.isControlPressed,
        altPressed = event.isAltPressed,
        metaPressed = event.isMetaPressed,
        shiftPressed = event.isShiftPressed;

  // Original key event
  final RawKeyEvent event;

  // Key event with "original" key without modifiers
  final LogicalKeyboardKey keyWithoutModifiers;

  final bool controlPressed;
  final bool altPressed;
  final bool metaPressed;
  final bool shiftPressed;
}

typedef KeyInterceptorHandler = bool Function(RawKeyEventEx event);

RawKeyEventEx _keyEventFromMessage(Map<String, dynamic> message) {
  final noModifiers = message['charactersIgnoringModifiersEx'] as String?;
  final event = RawKeyEvent.fromMessage(message);
  final data = event.data;
  if (noModifiers != null && data is RawKeyEventDataMacOs) {
    final newEvent = RawKeyEventDataMacOs(
      characters: noModifiers,
      charactersIgnoringModifiers: noModifiers,
      keyCode: data.keyCode,
      modifiers: data.modifiers,
    );
    return RawKeyEventEx(
        event: event, keyWithoutModifiers: newEvent.logicalKey);
  } else {
    return RawKeyEventEx(event: event, keyWithoutModifiers: event.logicalKey);
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
