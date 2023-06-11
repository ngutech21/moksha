import 'package:flutter/material.dart';
import 'package:qr_flutter/qr_flutter.dart';
import '../ffi.dart' if (dart.library.html) '../ffi_web.dart';

class MintPage extends StatelessWidget {
  const MintPage({super.key});

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Column(
        children: [MintWidget()],
      ),
    );
  }
}

class MintWidget extends StatefulWidget {
  const MintWidget({super.key});

  @override
  State<MintWidget> createState() => _MintWidgetState();
}

class _MintWidgetState extends State<MintWidget> {
  bool _isMinted = false;
  String amount = '';

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        _isMinted
            ? QrImageView(
                data: amount,
                version: QrVersions.auto,
                size: 200.0,
                backgroundColor: Colors.white,
                foregroundColor: Colors.black,
              )
            : const Text(
                'Not minted',
                style: TextStyle(fontSize: 20),
              ),
        TextField(
          keyboardType: TextInputType.number,
          decoration: const InputDecoration(
            border: OutlineInputBorder(),
            labelText: 'Amount',
          ),
          onChanged: (value) => setState(() {
            amount = value;
          }),
        ),
        ElevatedButton(
          onPressed: () {
            setState(() {
              _isMinted = true;
              print(amount);
            });
          },
          child: const Text('Mint'),
        ),
        ElevatedButton(
          onPressed: () async {
            var qr = await api.generateQrcode(amount: 6);
            print("QR$qr");
          },
          child: const Text('Call Future'),
        )
      ],
    );
  }
}
