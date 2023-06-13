import 'package:cashurs_wallet/ffi.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
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
  final _textController = TextEditingController();

  @override
  void dispose() {
    super.dispose();
    _textController.dispose();
  }

  @override
  void initState() {
    super.initState();

    _textController.addListener(() {
      // using Ashok's answer to format the text
      final regEx = RegExp(r'(\d{1,3})(?=(\d{3})+(?!\d))');
      matchFunc(Match match) => '${match[1]},';
      final text = _textController.text;

      _textController.value = _textController.value.copyWith(
        // we need to remove all the ',' from the values before reformatting
        // if you use other formatting values, remember to remove them here
        text: text.replaceAll(',', '').replaceAllMapped(regEx, matchFunc),
        // this will keep the cursor on the right as you type in values
        selection: TextSelection(
          baseOffset: text.length,
          extentOffset: text.length,
        ),
      );
    });
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.all(24),
      padding: const EdgeInsets.all(24),
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
                      style: const TextStyle(fontSize: 16),
                    ),
                  ],
                )
              : const Text(
                  'Pay a lighting invoice to mint cashu tokens',
                  style: TextStyle(fontSize: 20),
                ),
          Container(
            margin: const EdgeInsets.all(24),
            padding: const EdgeInsets.all(24),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              children: [
                const Spacer(),
                Visibility(
                    visible: !_isInvoiceCreated,
                    child: Flexible(
                      fit: FlexFit.loose,
                      child: Padding(
                        padding: const EdgeInsets.all(8),
                        child: TextField(
                          controller: _textController,
                          inputFormatters: [
                            LengthLimitingTextInputFormatter(9),
                            FilteringTextInputFormatter.digitsOnly,
                          ],
                          keyboardType: TextInputType.number,
                          decoration: const InputDecoration(
                            border: OutlineInputBorder(),
                            labelText: 'Amount (sats)',
                          ),
                          onChanged: (value) => setState(() {
                            amount = value;
                          }),
                        ),
                      ),
                    )),
                Visibility(
                    visible: !_isInvoiceCreated,
                    child: ElevatedButton(
                        onPressed: () async {
                          var result = await api.getMintPaymentRequest(
                              amount:
                                  int.parse(amount)); // use decimalTextfield
                          setState(() {
                            paymentRequest = result;
                            _isInvoiceCreated = true;
                          });

                          var mintedTokens = await api.mintTokens(
                              amount: int.parse(amount),
                              hash: paymentRequest!.hash);
                          setState(() {
                            paymentRequest = null;
                            _isInvoiceCreated = false;
                            amount = ''; // FIMXE clear textfield
                          });

                          if (!context.mounted) return;
                          ScaffoldMessenger.of(context).showSnackBar(SnackBar(
                            content: Column(
                                children: [Text('Minted $mintedTokens sats')]),
                            showCloseIcon: true,
                          ));
                        },
                        child: const Text('Mint tokens'))),
                const Spacer(),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
