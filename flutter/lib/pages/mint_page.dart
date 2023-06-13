import 'package:cashurs_wallet/ffi.dart';
import 'package:flutter/material.dart';
import 'package:qr_flutter/qr_flutter.dart';

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
  bool _isInvoiceCreated = false;
  String amount = '';
  FlutterPaymentRequest? paymentRequest;

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.all(24),
      child: Column(
        children: [
          _isInvoiceCreated
              ? Column(
                  children: [
                    QrImageView(
                      data: paymentRequest!.pr,
                      version: QrVersions.auto,
                      size: 200.0,
                      backgroundColor: Colors.white,
                      foregroundColor: Colors.black,
                    ),
                    SelectableText(
                      paymentRequest!.pr,
                      style: const TextStyle(fontSize: 18),
                    ),
                  ],
                )
              : const Text(
                  'Not minted',
                  style: TextStyle(fontSize: 20),
                ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: [
              Flexible(
                child: TextField(
                  keyboardType: TextInputType.number,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    labelText: 'Amount',
                  ),
                  onChanged: (value) => setState(() {
                    amount = value;
                  }),
                ),
              ),
              ElevatedButton(
                  onPressed: () async {
                    var result = await api.getMintPaymentRequest(
                        amount: int.parse(amount)); // use decimalTextfield
                    setState(() {
                      paymentRequest = result;
                      _isInvoiceCreated = true;
                    });

                    var mintedTokens = await api.mintTokens(
                        amount: int.parse(amount), hash: paymentRequest!.hash);
                    setState(() {
                      paymentRequest = null;
                      _isInvoiceCreated = false;
                      amount = ''; // FIMXE clear textfield
                    });

                    if (!context.mounted) return;
                    ScaffoldMessenger.of(context).showSnackBar(SnackBar(
                      content:
                          Column(children: [Text('Minted $mintedTokens sats')]),
                      showCloseIcon: true,
                    ));
                  },
                  child: const Text('Mint tokens')),
            ],
          ),
        ],
      ),
    );
  }
}
