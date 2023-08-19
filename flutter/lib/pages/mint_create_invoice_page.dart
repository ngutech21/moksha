// ignore_for_file: use_build_context_synchronously

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:moksha_wallet/generated/bridge_definitions.dart';
import 'package:moksha_wallet/main.dart';
import 'package:moksha_wallet/pages/util.dart';
import 'package:moksha_wallet/widgets/qr_viewer.dart';
import 'package:flutter/foundation.dart' show kIsWeb;

enum MintType { cashu, fedimint }

class MintCreateInvoicePage extends StatelessWidget {
  const MintCreateInvoicePage({super.key});

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Column(
        children: [MintWidget()],
      ),
    );
  }
}

class MintWidget extends ConsumerStatefulWidget {
  const MintWidget({super.key});

  @override
  ConsumerState<MintWidget> createState() => _MintWidgetState();
}

class _MintWidgetState extends ConsumerState<MintWidget> {
  bool _isInvoiceCreated = false;
  MintType? selectedMintType = MintType.cashu;
  String amount = '';
  String? paymentRequest;
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
      final regEx = RegExp(r'(\d{1,3})(?=(\d{3})+(?!\d))');
      matchFunc(Match match) => '${match[1]},';
      final text = _textController.text;
      _textController.value = _textController.value.copyWith(
        text: text.replaceAll(',', '').replaceAllMapped(regEx, matchFunc),
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
              ? QrViewer(paymentRequest: paymentRequest)
              : const Text(
                  "Pay a lighting invoice to mint tokens",
                  style: TextStyle(fontSize: 20),
                ),
          Container(
            margin: const EdgeInsets.all(24),
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Visibility(
                    visible: !_isInvoiceCreated && !kIsWeb,
                    child: DropdownButton<MintType>(
                        value: selectedMintType,
                        items: const [
                          DropdownMenuItem(value: MintType.cashu, child: Text("Cashu")),
                          DropdownMenuItem(value: MintType.fedimint, child: Text("Fedimint"))
                        ],
                        onChanged: (value) {
                          setState(() {
                            selectedMintType = value;
                          });
                        })),
                Visibility(
                    visible: !_isInvoiceCreated,
                    child: Flexible(
                      fit: FlexFit.loose,
                      child: Padding(
                        padding: const EdgeInsets.all(8),
                        child: Column(
                          children: [
                            TextField(
                              controller: _textController,
                              autofocus: true,
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
                          ],
                        ),
                      ),
                    )),
                Visibility(
                    visible: !_isInvoiceCreated,
                    child: ElevatedButton(
                        onPressed: () async {
                          if (amount == '' || amount == '0') {
                            showMessageSnackBar(context, 'Amount must be greater than 0');
                            return;
                          }

                          var cleanAmount = int.parse(amount.replaceAll(",", ""));

                          if (selectedMintType == MintType.cashu) {
                            try {
                              FlutterPaymentRequest cashuPaymentRequest = await api.getCashuMintPaymentRequest(amount: cleanAmount).first;

                              setState(() {
                                paymentRequest = cashuPaymentRequest.pr;
                                _isInvoiceCreated = true;
                              });

                              var mintedTokens = await api.cashuMintTokens(amount: cleanAmount, hash: cashuPaymentRequest.hash).first;
                              setState(() {
                                paymentRequest = null;
                                _isInvoiceCreated = false;
                                amount = ''; // FIXME clear textfield
                              });

                              updateCashuBalance(ref);

                              showMessageSnackBar(context, 'Minted ${formatSats(mintedTokens)} sats');
                            } catch (e) {
                              showErrorSnackBar(context, e, "Error creating invoice");
                              return;
                            }
                          } else if (selectedMintType == MintType.fedimint) {
                            try {
                              var fedimintPaymentRequest = await api.getFedimintPaymentRequest(amount: cleanAmount).first; // use decimalTextfield
                              setState(() {
                                paymentRequest = fedimintPaymentRequest.pr;
                                _isInvoiceCreated = true;
                              });

                              var mintedTokens = await api.fedimintMintTokens(amount: cleanAmount, operationId: fedimintPaymentRequest.operationId).first;
                              setState(() {
                                paymentRequest = null;
                                _isInvoiceCreated = false;
                                amount = ''; // FIXME clear textfield
                              });

                              updateFedimintBalance(ref);
                              showMessageSnackBar(context, 'Minted ${formatSats(mintedTokens)} sats');
                            } catch (e) {
                              showErrorSnackBar(context, e, "Error creating invoice");
                              return;
                            }
                          } else {
                            showMessageSnackBar(context, 'Select a mint type');
                            return;
                          }
                        },
                        child: const Text('Mint tokens'))),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
