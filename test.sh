#!/bin/bash

# Create directories if they don't exist
mkdir -p .tollgate/{programs,accounts/pool_config}

# Define the programs to dump
programs=(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA:spl_token.so"
  "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb:spl_token_2022.so"
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL:spl_ata.so"
  "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG:damm_v2.so"
  "strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUxfmKg5m:streamflow.so"
)

# Dump each program
for program in "${programs[@]}"; do
  id=${program%%:*}
  filename=${program##*:}
  filepath=".tollgate/programs/$filename"
  if ! [ -f "$filepath" ]; then
    echo "Dumping program $id to $filepath"
    solana program dump -u m "$id" "$filepath"
  else
    echo "File $filepath already exists, skipping"
  fi
done

# Define the accounts to dump
accounts=(
  "5SEpbdjFK5FxwTvfsGMXVQTD2v4M2c5tyRTxhdsPkgDw:streamflow_treasury.json"
  "wdrwhnCv4pzW8beKsbPa4S2UDZrXenjg16KJdKSpb5u:streamflow_withdrawor.json"
  "B743wFVk2pCYhV91cn287e1xY7f1vt4gdY48hhNiuQmT:streamflow_fee_oracle.json"

  "8CNy9goNQNLM4wtgRw528tUQGMKD3vSuFRZY2gLGLLvF:pool_config/0.json"
  "82p7sVzQWZfCrmStPhsG8BYKwheQkUiXSs2wiqdhwNxr:pool_config/1.json"
  "FzvMYBQ29z2J21QPsABpJYYxQBEKGsxA6w6J2HYceFj8:pool_config/2.json"
  "EQbqYxecZuJsVt6g5QbKTWpNWa3QyWQE5NWz5AZBAiNv:pool_config/3.json"
  # "9RuAyDH81GB9dhks6MzHva2objQJxHvqRRfyKKdfmkxk:pool_config/4.json"
  # "GqRo1PG5KZc4QqZn1RCcnEGC8E7yRscHaW1fQp9St9Lz:pool_config/5.json"
  # "3KLdspUofc75aaEAJdBo1o6D6cyzXJVtGB8PgpWJEiaR:pool_config/6.json"
  # "9xKsCsiv8eeBohobb8Z1snLZzVKKATGqmY69vJHyCzvu:pool_config/7.json"
  # "EVRn9bAekgZsVVAHt25AUjA7qpKh4ac7uUMpoSGqgS5U:pool_config/8.json"
  # "7BJfgt3ahTtCfXkPMRbS6YneR92JuwsU1dyayhmNBL11:pool_config/9.json"
  # "ABWG34FJMHaWSwP2uJrX2S6dKXDmz93MCVSBk9BKZHrs:pool_config/10.json"
  # "HrBAyo6rf8i6dF8S8kh6QsjTmesmFhDoHvwSrsUHKdbX:pool_config/11.json"
  # "DXoY3hDAuvQudWTjpepSJ1bn1yd6jovuvPweHwc1e83P:pool_config/12.json"
  # "69CwWBvDBGvZ9P6bB9UnMwnDcQ136UFuDn2UEZ7Rb5We:pool_config/13.json"
  # "BDAbqqPRRg44tsDUEUPFjVaReX1mavngTc9H9SFPDo6F:pool_config/14.json"
  # "EZDtwCGcoe3f7BWFxaMrYDTq2WZMrcBZbUktoBKYvYiM:pool_config/15.json"
  # "341nQvGfd3b6HXMEaMafZwk5DkHmrZDh7Q2j4BbTCHyk:pool_config/16.json"
  # "6Vt8KLaRA1T1aecBVca4VPpJi27oAPLDZUPbrUocEN12:pool_config/17.json"
  # "F7xJjVwqvVBoAkYV3TdZesu4ckwzzVQEebaPiZVqT4Ly:pool_config/18.json"
  # "84tnQ4tQ4N8QGwoEEXcCdWfjSeuL8SfNjsdZWLZ9UiY4:pool_config/19.json"
  # "HKqzgVzaKkX7NPXyVNWuVegtgujkiJ2ZLvpszR1iZjhd:pool_config/20.json"
  # "5LWDYbiD3LwEhAd3eeHpjJscACKo4XdTDzycdHFSBCvE:pool_config/21.json"
  # "EcfqEkLSeGzDtZrTJWcbDxptfR2nWfX6cjJLFkgttwY6:pool_config/22.json"
  # "GkC7zppTNPjBeoZfKCR9ExbNSycpwi5VphvktqpyPdx3:pool_config/23.json"
  # "7tcR7XawzXCSAtAavjVYU2Rx5RK8mE9rsyrErfYRpkw4:pool_config/24.json"
  # "2yAJha5NVgq5mEitTUvdWSUKrcYvxAAc2H6rPDbEQqSu:pool_config/25.json"
  # "G9EUpuBrDZHQAeaifkx5xbiajAGbB6HHJ4xcVmZyd3eQ:pool_config/26.json"
  # "3z9HHXyWEXc7L3EPEQ5mN8cPoq9wBZr8y2bRiEwUu9u2:pool_config/27.json"
  # "9YmoetVvZx1vrfJ9fD8X5YG3FQXREK6ZiPzRghP33Wbf:pool_config/28.json"
  # "Ha2bAcxbLrFr5RiugBgeJVLx1JE7gq16rzAuqUED1v3f:pool_config/29.json"
  # "GtDtC9gJEAyMje1AW1McMoAFpGqcfYPwnGMBJ3VLS54Q:pool_config/30.json"
  # "4A95FoEsswvuCEDSFnd8uXBgdkQPqzrjJv2roev8c9mm:pool_config/31.json"
  # "CsPBLWzLWTJ3p8PG28zQ31Eq3dPpw1wV55JxpRYzdVxg:pool_config/32.json"
  # "GXZLjqmebpsy74vqTD6DqSTugTKVwoTi8fZwLAXBsMNN:pool_config/33.json"
  # "AeLtDKgw3XnXbr3Kgfbcb7KiZULVCQ5mXaFDiG9n7EgW:pool_config/34.json"
  # "7f8zQkCTmEE2yPjKoEGpWxcTaw6VYcjB1P3DxnpfFNCc:pool_config/35.json"
  # "G6Sukhgcmaf32PucWqCTMHn4jtWjE2TZTk59eQPSVKsy:pool_config/36.json"
  # "CLL5Wi7pi9SwHiSwtMyz1xbX6HDq3defpotWUFxwu4oj:pool_config/37.json"
  # "9jma77W3ZsJXPude5tnmC51EhMjKHzwsHCz1gFmbfJBc:pool_config/38.json"
  # "BdYG3xCpAYPYPnksHGhVfEMHk3gqt1Y7uqdWUsSUzf4y:pool_config/39.json"
  # "uPhetWqk4hhf9swL8xdbABfmGh4GyQ9nVAeYpvnC6pb:pool_config/40.json"
  # "TBuzuEMMQizTjpZhRLaUPavALhZmD8U1hwiw1pWSCSq:pool_config/41.json"
  # "3z9HHXyWEXc7L3EPEQ5mN8cPoq9wBZr8y2bRiEwUu9u2:pool_config/42.json"
  # "GGD1oNYU62ux15XXpSMeoKcfznHTtu85qLeKrhY7MMMZ:pool_config/43.json"
  # "4hKGzanVuqCaVHbXm9rXnYJeJvzWcq5HDogKcrVYh4gP:pool_config/44.json"
  # "G4Y8SphEjVCESkodbFU7sjgaPZRykD55dabccDa8Lv7M:pool_config/45.json"
  # "GnERyyZgr9JdZ5dCFC46APkcPNZdUQgrJmRqoeBX55dg:pool_config/46.json"
  # "EcfqEkLSeGzDtZrTJWcbDxptfR2nWfX6cjJLFkgttwY6:pool_config/47.json"
  # "C11DxNAH4NBGNHGzTCq9ZUcJrVJ9dEG5CLwSmis3Y6HJ:pool_config/48.json"
  # "2rbDaKQjxiFgMsYoQxLRaPvtXuLFUUQmypN5bYmJqPjY:pool_config/49.json"
  # "7gE2roG5cBM5hpqDQvQ2J7ZsQE4CqM6eE8sKQ6NTaqRS:pool_config/50.json"
  # "DJN8YHxQKZnF7bL2GwuKNB2UcfhKCqRspfLe7YYEN3rr:pool_config/51.json"
  # "6Fs8KLaRA1T1aecBVca4VPpJi27oAPLDZUPbrUocEN12:pool_config/52.json"
  # "7SDjNZxM4rGNdYF3MyAkDetZ2TNFUxdDUGGE1C3kCeAd:pool_config/53.json"
  # "2yAJha5NVgq5mEitTUvdWSUKrcYvxAAc2H6rPDbEQqSu:pool_config/54.json"
  # "E4VmzCAgMN2GGAhiRipGfJhaP411Ap7YFy8WmnnV2CKs:pool_config/55.json"
  # "A4JGKvpXKGfpkSgBSLmR7obESYhqqaeVEve71nWmS4zU:pool_config/56.json"
  # "7tcR7XawzXCSAtAavjVYU2Rx5RK8mE9rsyrErfYRpkw4:pool_config/57.json"
  # "GkC7zppTNPjBeoZfKCR9ExbNSycpwi5VphvktqpyPdx3:pool_config/58.json"
  # "GGD1oNYU62ux15XXpSMeoKcfznHTtu85qLeKrhY7MMMZ:pool_config/59.json"
  # "4hKGzanVuqCaVHbXm9rXnYJeJvzWcq5HDogKcrVYh4gP:pool_config/60.json"
  # "G4Y8SphEjVCESkodbFU7sjgaPZRykD55dabccDa8Lv7M:pool_config/61.json"
  # "GnERyyZgr9JdZ5dCFC46APkcPNZdUQgrJmRqoeBX55dg:pool_config/62.json"
  # "C11DxNAH4NBGNHGzTCq9ZUcJrVJ9dEG5CLwSmis3Y6HJ:pool_config/63.json"
  # "2rbDaKQjxiFgMsYoQxLRaPvtXuLFUUQmypN5bYmJqPjY:pool_config/64.json"
  # "7gE2roG5cBM5hpqDQvQ2J7ZsQE4CqM6eE8sKQ6NTaqRS:pool_config/65.json"
  # "DJN8YHxQKZnF7bL2GwuKNB2UcfhKCqRspfLe7YYEN3rr:pool_config/66.json"
  # "6Fs8KLaRA1T1aecBVca4VPpJi27oAPLDZUPbrUocEN12:pool_config/67.json"
  # "7SDjNZxM4rGNdYF3MyAkDetZ2TNFUxdDUGGE1C3kCeAd:pool_config/68.json"
  # "2yAJha5NVgq5mEitTUvdWSUKrcYvxAAc2H6rPDbEQqSu:pool_config/69.json"
  # "E4VmzCAgMN2GGAhiRipGfJhaP411Ap7YFy8WmnnV2CKs:pool_config/70.json"
  # "A4JGKvpXKGfpkSgBSLmR7obESYhqqaeVEve71nWmS4zU:pool_config/71.json"
)

# Dump each account
for account in "${accounts[@]}"; do
  id=${account%%:*}
  filename=${account##*:}
  filepath=".tollgate/accounts/$filename"
  if ! [ -f "$filepath" ]; then
    echo "Dumping account $id to $filepath"
    solana account -u m --output json-compact --output-file "$filepath" "$id"
  else
    echo "File $filepath already exists, skipping"
  fi
done

# Run cargo test
cargo test -- --test-threads=1 --show-output
