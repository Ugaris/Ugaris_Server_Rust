use thiserror::Error;

pub const CHARACTER_NAME_SIZE: usize = 40;
pub const PASSWORD_SIZE: usize = 16;
pub const LOGIN_BLOCK_SIZE: usize = CHARACTER_NAME_SIZE + PASSWORD_SIZE + 16;
pub const UGARIS_VENDOR_PREFIX: u32 = 0x8fd46100;

const SECRET: [[u8; PASSWORD_SIZE]; 4] = [
    *b"\0cgf\0de8etzdf\0dx",
    *b"jrfa\0v7d\0drt\0edm",
    *b"t6zh\0dlr\0fu4dms\0",
    *b"jkdm\0u7z5g\0j77\0g",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginBlock {
    pub name: String,
    pub password: String,
    pub vendor: u32,
    pub client_version: Option<u8>,
    pub his_ip: u32,
    pub our_ip: u32,
    pub unique: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LoginError {
    #[error("login block is too short: {0} bytes")]
    TooShort(usize),
    #[error("character name is empty")]
    EmptyName,
}

impl LoginBlock {
    pub fn parse(input: &[u8]) -> Result<Self, LoginError> {
        if input.len() < LOGIN_BLOCK_SIZE {
            return Err(LoginError::TooShort(input.len()));
        }

        let name_bytes = &input[..CHARACTER_NAME_SIZE];
        let name = c_string(name_bytes).trim().to_string();
        if name.is_empty() {
            return Err(LoginError::EmptyName);
        }

        let mut password = [0_u8; PASSWORD_SIZE];
        password.copy_from_slice(&input[CHARACTER_NAME_SIZE..CHARACTER_NAME_SIZE + PASSWORD_SIZE]);
        decrypt_password(name_bytes, &mut password);

        let vendor_offset = CHARACTER_NAME_SIZE + PASSWORD_SIZE;
        let vendor =
            u32::from_le_bytes(input[vendor_offset..vendor_offset + 4].try_into().unwrap());
        let his_ip = u32::from_le_bytes(
            input[vendor_offset + 4..vendor_offset + 8]
                .try_into()
                .unwrap(),
        );
        let our_ip = u32::from_le_bytes(
            input[vendor_offset + 8..vendor_offset + 12]
                .try_into()
                .unwrap(),
        );
        let unique = u32::from_le_bytes(
            input[vendor_offset + 12..vendor_offset + 16]
                .try_into()
                .unwrap(),
        );
        let client_version =
            ((vendor & 0xffffff00) == UGARIS_VENDOR_PREFIX).then_some((vendor & 0xff) as u8);

        Ok(Self {
            name,
            password: c_string(&password),
            vendor,
            client_version,
            his_ip,
            our_ip,
            unique,
        })
    }
}

pub fn decrypt_password(name: &[u8], password: &mut [u8; PASSWORD_SIZE]) {
    let selector = name.get(1).copied().unwrap_or_default() as usize % 4;
    for i in 0..PASSWORD_SIZE {
        password[i] ^= SECRET[selector][i] ^ name.get(i % 3).copied().unwrap_or_default();
    }
}

fn c_string(input: &[u8]) -> String {
    let end = input
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(input.len());
    String::from_utf8_lossy(&input[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decrypt_is_symmetric_for_legacy_password_block() {
        let mut password = *b"secret\0\0\0\0\0\0\0\0\0\0";
        let original = password;
        let name = b"Tester\0";
        decrypt_password(name, &mut password);
        decrypt_password(name, &mut password);
        assert_eq!(password, original);
    }

    #[test]
    fn short_or_garbage_login_blocks_never_panic() {
        // Attacker-controlled bytes: every undersized block must be
        // rejected with an error, never a slice/index panic.
        for len in 0..LOGIN_BLOCK_SIZE {
            assert_eq!(
                LoginBlock::parse(&vec![0xff_u8; len]),
                Err(LoginError::TooShort(len))
            );
        }
        // Full-size all-0xff block parses without panicking.
        let _ = LoginBlock::parse(&[0xff_u8; LOGIN_BLOCK_SIZE]);
        // Short names (< 3 bytes) must not panic the XOR decrypt.
        for name in [&b""[..], b"a", b"ab"] {
            let mut password = [0xff_u8; PASSWORD_SIZE];
            decrypt_password(name, &mut password);
        }
    }

    #[test]
    fn parses_login_block_and_client_version() {
        let mut block = [0_u8; LOGIN_BLOCK_SIZE];
        block[..6].copy_from_slice(b"Tester");
        let mut encrypted = *b"secret\0\0\0\0\0\0\0\0\0\0";
        decrypt_password(&block[..CHARACTER_NAME_SIZE], &mut encrypted);
        block[CHARACTER_NAME_SIZE..CHARACTER_NAME_SIZE + PASSWORD_SIZE].copy_from_slice(&encrypted);
        block[56..60].copy_from_slice(&(UGARIS_VENDOR_PREFIX | 3).to_le_bytes());

        let parsed = LoginBlock::parse(&block).unwrap();
        assert_eq!(parsed.name, "Tester");
        assert_eq!(parsed.password, "secret");
        assert_eq!(parsed.client_version, Some(3));
    }
}
