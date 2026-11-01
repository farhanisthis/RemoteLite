use remotelite::crypto::{derive_session_key, generate_entropy, SecureChannel};

#[test]
fn test_ephemeral_key_derivation() {
    let client_entropy1 = generate_entropy();
    let host_entropy1 = generate_entropy();
    let key1 = derive_session_key(&client_entropy1, &host_entropy1);

    let client_entropy2 = generate_entropy();
    let host_entropy2 = generate_entropy();
    let key2 = derive_session_key(&client_entropy2, &host_entropy2);

    assert_ne!(key1, key2);
    assert_eq!(key1.len(), 32);
}

#[test]
fn test_secure_channel_encryption_and_tamper_detection() {
    let key = [0x42u8; 32];
    let host_channel = SecureChannel::new(&key, 0x01);
    let client_channel = SecureChannel::new(&key, 0x02);

    let secret_data = b"Remote Desktop Encrypted Keystroke: Hello World";

    // Host encrypts data
    let encrypted = host_channel.encrypt(secret_data).unwrap();

    // Plaintext is not exposed in ciphertext
    assert!(!encrypted.windows(secret_data.len()).any(|w| w == secret_data));

    // Client decrypts
    let decrypted = client_channel.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted, secret_data);

    // Tampering 1 bit causes authentication failure
    let mut corrupted = encrypted.clone();
    corrupted[15] ^= 0x01;
    assert!(client_channel.decrypt(&corrupted).is_err());
}
