import 'package:mutex/mutex.dart';

// Forked from Mutex package to allow protect to return a value
class Mutex {
  // Implemented as a ReadWriteMutex that is used only with write locks.
  final ReadWriteMutex _rwMutex = ReadWriteMutex();

  /// Indicates if a lock has been acquired and not released.
  bool get isLocked => (_rwMutex.isLocked);

  /// Acquire a lock
  ///
  /// Returns a future that will be completed when the lock has been acquired.
  ///
  /// Consider using the convenience method [protect], otherwise the caller
  /// is responsible for making sure the lock is released after it is no longer
  /// needed. Failure to release the lock means no other code can acquire the
  /// lock.

  Future acquire() => _rwMutex.acquireWrite();

  /// Release a lock.
  ///
  /// Release a lock that has been acquired.

  void release() => _rwMutex.release();

  /// Convenience method for protecting a function with a lock.
  ///
  /// A lock is acquired before invoking the [criticalSection] function.
  /// If the critical section returns a Future, it waits for it to be completed
  /// before the lock is released. The lock is always released
  /// (even if the critical section throws an exception).
  ///
  /// Returns a Future that completes after the lock is released.

  Future<T> protect<T>(Future<T> Function() criticalSection) async {
    await acquire();
    T res;
    try {
      res = await criticalSection();
    } finally {
      release();
    }
    return res;
  }
}
