import 'package:flutter/services.dart';
import 'key_interceptor.dart';

class AcceleratorKey {
  AcceleratorKey(this.key, this.label);

  final LogicalKeyboardKey key;
  final String label;
}

class Accelerator {
  const Accelerator({
    this.key,
    this.alt = false,
    this.control = false,
    this.meta = false,
    this.shift = false,
  });

  final AcceleratorKey? key;
  final bool alt;
  final bool control;
  final bool meta;
  final bool shift;

  Accelerator operator +(dynamic that) {
    if (that is num) {
      that = '$that';
    }

    if (that is String) {
      assert(that.codeUnits.length == 1);
      final lower = that.toLowerCase();
      return this +
          Accelerator(
              key: AcceleratorKey(
                  _keyForCodeUnit(lower.codeUnits[0]), that.toUpperCase()),
              shift: lower != that);
    } else if (that is Accelerator) {
      return Accelerator(
          key: that.key ?? key,
          alt: alt || that.alt,
          shift: shift || that.shift,
          control: control || that.control,
          meta: meta || that.meta);
    } else {
      throw ArgumentError(
          'Argument must be String, Accelerator or single digit number');
    }
  }

  bool matches(RawKeyEventEx event) {
    return event.altPressed == alt &&
        event.controlPressed == control &&
        event.metaPressed == meta &&
        event.shiftPressed == shift &&
        event.keyWithoutModifiers == key?.key;
  }

  LogicalKeyboardKey _keyForCodeUnit(int codeUnit) {
    final keyId = LogicalKeyboardKey.unicodePlane |
        (codeUnit & LogicalKeyboardKey.valueMask);
    return LogicalKeyboardKey.findKeyByKeyId(keyId) ??
        LogicalKeyboardKey(
          keyId,
          keyLabel: String.fromCharCode(codeUnit),
          debugName: null,
        );
  }

  dynamic serialize() => key != null
      ? {
          'label': key!.label,
          'alt': alt,
          'shift': shift,
          'meta': meta,
          'control': control,
        }
      : null;
}
