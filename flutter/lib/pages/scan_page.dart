import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:moksha_wallet/main.dart';
import 'package:go_router/go_router.dart';

class ScanPage extends StatelessWidget {
  ScanPage({super.key});

  final MobileScannerController cameraController = MobileScannerController();

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Scan QR code'),
        actions: [
          IconButton(
            color: Colors.white,
            icon: ValueListenableBuilder(
              valueListenable: cameraController.torchState,
              builder: (context, state, child) {
                switch (state) {
                  case TorchState.off:
                    return const Icon(Icons.flash_off, color: Colors.grey);
                  case TorchState.on:
                    return const Icon(Icons.flash_on, color: Colors.yellow);
                }
              },
            ),
            iconSize: 32.0,
            onPressed: () => cameraController.toggleTorch(),
            tooltip: "Toggle torch",
          ),
          IconButton(
            color: Colors.white,
            icon: ValueListenableBuilder(
              valueListenable: cameraController.cameraFacingState,
              builder: (context, state, child) {
                switch (state) {
                  case CameraFacing.front:
                    return const Icon(Icons.camera_front);
                  case CameraFacing.back:
                    return const Icon(Icons.camera_rear);
                }
              },
            ),
            iconSize: 32.0,
            onPressed: () => cameraController.switchCamera(),
            tooltip: "Switch camera",
          ),
        ],
      ),
      body: MobileScanner(
        // fit: BoxFit.contain,
        controller: cameraController,
        onDetect: (capture) {
          final List<Barcode> barcodes = capture.barcodes;
          for (final barcode in barcodes) {
            debugPrint('Barcode found! ${barcode.rawValue} ${barcode.type}');
            print('Barcode type ${barcode.type} ${barcode.rawValue}');
            if (barcode.type == BarcodeType.text && barcode.rawValue!.startsWith("ln")) {
              var decodedInvoice = api.decodeInvoice(invoice: barcode.rawValue!);
              print('decodedInvoice ${decodedInvoice.amountSats} ${decodedInvoice.expiryTime} ${decodedInvoice.pr}');
              //context.go(Uri(path: '/pay', queryParameters: {'pr': barcode.rawValue!}).toString());
              context.goNamed("pay", extra: decodedInvoice);
            }
          }
        },
      ),
    );
  }
}
