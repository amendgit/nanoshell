import 'dart:async';
import 'dart:convert';

import 'package:flutter/cupertino.dart';
import 'package:flutter/services.dart';

const channel = BasicMessageChannel('nanoshell/keyevent', BinaryCodec());

typedef KeyInterceptorHandler = bool Function(RawKeyEvent event);

class KeyInterceptor {
  KeyInterceptor._() {
    channel.setMessageHandler(_onMessage);
  }

  Future<ByteData> _onMessage(ByteData? message) {
    final string = StringCodec().decodeMessage(message) ?? '';
    final keyMessage = json.decode(string);
    final event = RawKeyEvent.fromMessage(keyMessage);

    for (final handler in List<KeyInterceptorHandler>.from(_handlers)) {
      if (handler(event)) {
        final res = <String, dynamic>{'handled': true};
        final encoded = StringCodec().encodeMessage(json.encode(res));
        return Future.value(encoded);
      }
    }

    final completer = Completer<ByteData>();
    WidgetsBinding.instance?.defaultBinaryMessenger
        .handlePlatformMessage('flutter/keyevent', message, (data) {
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
