# Threat model

## Protected

A person gets the encrypted snapshot but does not know the password.

A person modifies the encrypted payload or authenticated header.

Malformed imports are rejected before restore.

## Not protected

A compromised machine can read tabs before encryption or after decryption.

Malware with access to the clipboard can read a copied snapshot string.

A weak password is still a weak password. Argon2id only makes guessing more expensive.

A user who loses the password loses the snapshot.

## Network

Level 1 does not need the network.

No snapshot, password or derived key is sent anywhere by TabSnap.
