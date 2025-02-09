

pub fn verify_proof(proof: Vec<[u8; 32]>, root: [u8; 32], leaf: [u8; 32]) {
    let mut current_hash = leaf;
    for node in proof.into_iter() {
        // msg!("Current Hash {:02X?}", current_hash);
        // Nodes are arranged so the smaller one is on the left.
        if current_hash <= node {
            current_hash = anchor_lang::solana_program::keccak::hashv(&[&current_hash, &node]).0;
        } else {
            current_hash = anchor_lang::solana_program::keccak::hashv(&[&node, &current_hash]).0;
        }
    }
    // msg!("Last Hash {:02X?}", current_hash);
    // msg!("Root {:02X?}", root);
    assert_eq!(current_hash, root)
}