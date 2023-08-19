import 'package:flutter/material.dart';
import 'package:qr_flutter/qr_flutter.dart';
import 'package:share_plus/share_plus.dart';
import 'package:flutter/services.dart';

class QrViewer extends StatelessWidget {
  const QrViewer({
    super.key,
    required this.paymentRequest,
  });

  final String? paymentRequest;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        QrImageView(
          data: paymentRequest!,
          version: QrVersions.auto,
          size: 200.0,
          backgroundColor: Colors.white,
          eyeStyle: const QrEyeStyle(
            eyeShape: QrEyeShape.square,
            color: Colors.black,
          ),
        ),
        Container(
          padding: const EdgeInsets.all(8),
          margin: const EdgeInsets.all(8),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Spacer(),
              ElevatedButton(
                  onPressed: () {
                    Clipboard.setData(
                      ClipboardData(
                        text: paymentRequest!,
                      ),
                    );

                    if (!context.mounted) return;
                    ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
                      content: Column(children: [
                        Text('Copied invoice to clipboard'),
                      ]),
                      showCloseIcon: true,
                    ));
                  },
                  child: const Text('Copy')),
              const SizedBox(width: 8),
              ElevatedButton(
                  onPressed: () {
                    Share.share(paymentRequest!);
                  },
                  child: const Text('Share')),
              const Spacer(),
            ],
          ),
        )
      ],
    );
  }
}
