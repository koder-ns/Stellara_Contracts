// Simple illustrative circuit for proving accredited-investor status
// without revealing any additional personal information.
pragma circom 2.0.0;

template Accredited() {
  // Input signal:
  // 1 = accredited investor, 0 = not accredited
  signal input accredited;

  // Output signal indicating whether the claim is valid
  signal output out;

  // Returns 1 when accredited == 1, otherwise 0
  out <== accredited == 1;
}

// Main circuit entry point
component main = Accredited();