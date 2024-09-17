import * as anchor from "@coral-xyz/anchor";
import type { HelloAnchor } from "../target/types/hello_anchor";

describe("Test", () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.HelloAnchor as anchor.Program<HelloAnchor>;
  
  it("initialize", async () => {
    // Generate keypair for the new account

  });
});
