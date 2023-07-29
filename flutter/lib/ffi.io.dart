import 'dart:io';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';
import 'generated/bridge_generated.io.dart';

const base = 'native';
final path = Platform.isWindows ? '$base.dll' : 'lib$base.so';
final dylib = loadLibForFlutter(path);
final api = NativeImpl(dylib);
