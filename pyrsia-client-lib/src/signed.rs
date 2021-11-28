extern crate anyhow;
extern crate detached_jws;
extern crate openssl;
extern crate serde;
extern crate serde_jcs;
extern crate serde_json;

use std::option::Option;

use anyhow::{Context, Result};
use detached_jws::{DeserializeJwsWriter, SerializeJwsWriter};
use openssl::pkey::{PKey, Private};
use openssl::{
    hash::MessageDigest,
    pkey::PKeyRef,
    rsa::{Padding, Rsa},
    sign::{Signer, Verifier},
};
use serde::{Deserialize, Serialize};

/// An enumeration of the supported signature algorithms
#[derive(Deserialize, Serialize)]
pub enum SignatureAlgorithms {
    RsaPkcs1Sha512,
    RsaPkcs1Sha3_512,
}

// The default size for RSA keys
const DEFAULT_RSA_KEY_SIZE: u32 = 4096;

/// An instance of this struct is created to hold a key pair
#[derive(Deserialize, Serialize)]
pub struct SignatureKeyPair {
    pub signature_algorithm: SignatureAlgorithms,
    pub private_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

pub fn create_key_pair(
    signature_algorithm: SignatureAlgorithms,
) -> Result<SignatureKeyPair, anyhow::Error> {
    match signature_algorithm {
        SignatureAlgorithms::RsaPkcs1Sha3_512 | SignatureAlgorithms::RsaPkcs1Sha512 => {
            let rsa_private: Rsa<Private> = Rsa::generate(DEFAULT_RSA_KEY_SIZE)?;
            Ok(SignatureKeyPair {
                signature_algorithm: signature_algorithm,
                private_key: rsa_private.private_key_to_der()?,
                public_key: rsa_private.public_key_to_der()?,
            })
        }
    }
}

/// This trait should be implemented by all structs that contain signed data. Structs that implement
/// this trait should be annotated with
/// `#[derive(Serialize, Deserialize)]`
///
/// Pyrsia needs to manage a variety of data related to software artifacts. It will store all of
/// this data as JSON.  The reason for using JSON is to promote interoperability. If Pyrsia is
/// successful people will write their own implementations of Pyrsia nodes. For this reason, we
/// choose standard [JSON](https://www.json.org/json-en.html) .
///
/// All JSON that Pyrsia manages must be signed so that we can attribute it to a source and be
/// confident that it has not been modified since it was signed. Because we are using JSON based
/// signatures, when we deserialize JSON to a struct, to be considered signed, the struct must
/// contain a reference to the JSON it was deserialized from, so we can still verify the signature.
///
/// Methods that modify the contents of a signed struct should discard its associated JSON by
/// calling the clear_json method, since the JSON no longer matches that struct.
///
/// Given the above description of the purposes of the `Signed` trait, the descriptions of its
/// methods should be understood in this context.
///
/// It is recommended for consistency that structs that implement this trait are declared
/// like this with a field named `__json` to refer to the struct's json string:
/// ```
/// //noinspection NonAsciiCharacters
/// #[derive(Serialize, Deserialize, Debug)]
/// struct Foo<'a> {
///   foo: &'a str,
///   bar: u32,
///   #[serde(skip)]
///   π_json: Option<String>
/// }
/// ```
pub trait Signed<'a>: Deserialize<'a> + Serialize {
    /// Return as a string the signed JSON associated with this struct. Returns None if there no
    /// signed JSON is currently associated with the struct.
    fn json(&self) -> Option<String>;

    /// Remove the JSON string from the struct. This should be called by setter methods that modify
    /// the contents of the struct.
    fn clear_json(&mut self);

    /// Set the JSON string associated with this struct.
    ///
    /// This method should be private. It should only be called from the other methods of this
    /// trait.
    fn set_json(&mut self, _json: &str);

    /// Create a struct of type `T` from the contents of the given JSON string.
    ///
    /// Return the created struct if there is an error.
    fn from_json_string<T>(_json: &str) -> Result<T, anyhow::Error>
    where
        T: Signed<'a>,
    {
        todo!()
    }

    /// If this struct does not have an associated JSON representation then create it and pass it to
    /// the `set_json` method.
    ///
    /// Add a signature to the JSON using the contents of the given key pair.
    /// * signature_algorithm — The signature algorithm to use for signing. Must be compatible with the private key.
    /// * private_key — The der encoded private key to use for signing.
    fn sign(
        &mut self,
        signature_algorithm: SignatureAlgorithms,
        private_key: &Vec<u8>,
    ) -> Result<(), anyhow::Error> {
        let _unsigned_json: String = serde_jcs::to_string(self)?;
        with_signer(signature_algorithm, private_key, |signer| todo!())
    }

    // TODO Add a way to add an expiration time, role and other attributes to signatures.

    /// Verify the signature(s) of this struct's associated JSON.
    ///
    /// Return an error if any of the signatures are not valid.
    fn verify_signature(&self) -> Result<(), anyhow::Error> {
        todo!()
    }

    // TODO add a method to get the details of the signatures in this struct's associated JSON.
}

fn with_signer<'a>(
    signature_algorithm: SignatureAlgorithms,
    der_private_key: &[u8],
    signing_function: fn(Signer) -> Result<(), anyhow::Error>,
) -> Result<(), anyhow::Error> {
    let private_key: Rsa<Private> = Rsa::private_key_from_der(der_private_key)?;
    let kp: PKey<Private> = PKey::from_rsa(private_key)?;
    let mut signer = match signature_algorithm {
        SignatureAlgorithms::RsaPkcs1Sha512 => {
            Signer::new(MessageDigest::sha512(), &kp).context("Problem using key pair")
        }
        SignatureAlgorithms::RsaPkcs1Sha3_512 => {
            Signer::new(MessageDigest::sha3_512(), &kp).context("Problem using key pair")
        }
    }?;
    signer.set_rsa_padding(Padding::PKCS1_PSS)?;
    signing_function(signer)
}

/// Lightweight JSON parser to identify the portion of a slice before and after a value, so that the
/// value can easily be replaced.
mod json_parser {
    use anyhow::anyhow;

    pub enum JsonPathElement<'a> {
        Field(&'a str),
        Index(usize)
    }

    // Given a string slice that contains JSON and the path of a value, this returns three smaller
    // slices that are the characters before a specified value, the characters that comprise the value
    // and the characters after the value.
    pub fn parse<'a>(json: &'a str, path: &Vec<JsonPathElement>) -> Result<(&'a str, &'a str, &'a str), anyhow::Error> {
        let start_of_target: usize = 0;
        let end_of_target: usize = 0;
        //parse_value(&start_of_target, &end_of_target, path, &json)?;
        if end_of_target <= start_of_target {
            return Err(anyhow!(format!("Did not find {}", path_to_str(path))))
        }
        Ok((&json[..(start_of_target-1)], &json[start_of_target..end_of_target], &json[end_of_target+1 ..]))
    }

    pub fn path_to_str(path: &Vec<JsonPathElement>) -> String {
        let mut s = String::from("path[");
        if !path.is_empty() {
            path_element_to_str(&mut s, &path[0]);
            for path_element in path[1..].iter() {
                s.push_str("\",");
                path_element_to_str(&mut s, path_element);
            }
        }
        s.push_str("]");
        s
    }

    fn path_element_to_str(s: &mut String, path_element: &JsonPathElement) {
        match path_element {
            JsonPathElement::Field(field_name) => {
                s.push_str("field:\"");
                s.push_str(field_name);
            }
            JsonPathElement::Index(index) => {
                s.push_str(index.to_string().as_str());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use super::*;
    use json_parser::*;

    //noinspection NonAsciiCharacters
    #[derive(Serialize, Deserialize)]
    struct Foo<'a> {
        foo: &'a str,
        bar: u32,
        #[serde(skip)]
        π_json: Option<String>,
    }

    #[test]
    fn path_to_string_test() {
        let path = vec![JsonPathElement::Field("__signature"), JsonPathElement::Index(4)];
        assert_eq!("path[field:\"__signature\",4]", path_to_str(&path))
    }

    #[test]
    fn parse_failures() -> Result<(), anyhow::Error> {
        let object_path = vec![JsonPathElement::Field("__signature")];
        let object_json = r#"{"a":"x","b":"y"}"# ;
        let index_json = "[1,3,7]";
        match parse(object_json, &object_path) {
            Ok(_) => return Err(anyhow!("Not-found field did not produce an error")),
            Err(_) => {}
        };
        match parse(index_json, &object_path) {
            Ok(_) => return Err(anyhow!("Not-found field did not produce an error")),
            Err(_) => {}
        };
        let index_path = vec![JsonPathElement::Index(4)];
        match parse(object_json, &index_path) {
            Ok(_) => return Err(anyhow!("Not-found index did not produce an error")),
            Err(_) => {}
        };
        match parse(index_json, &index_path) {
            Ok(_) => return Err(anyhow!("Not-found index did not produce an error")),
            Err(_) => {}
        };
        Ok(())
    }

    #[test]
    fn happy_path_for_signing() -> Result<(), anyhow::Error> {
        let key_pair: SignatureKeyPair =
            crate::signed::create_key_pair(SignatureAlgorithms::RsaPkcs1Sha3_512)?;

        // create a key pair for other signing types to see that they succeed
        super::create_key_pair(SignatureAlgorithms::RsaPkcs1Sha512)?;

        Ok(())
    }
}