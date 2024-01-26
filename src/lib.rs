use crate::mconfig::MConfig;

pub mod mconfig;

// demo function
pub fn demo() {
    let mut mcnf = MConfig::builder().secret("TACOS").try_build().unwrap();
    mcnf.try_insert("Hello".to_string(), Some("World".to_string())).expect("Hello failed");
    mcnf.try_insert("Bye".to_string(), None).expect("Bye failed");

    // Convert it to a vec
    let mcv = mcnf.to_vec();
    println!("{:?}", mcv.len());

    // Make a new one using the vec
    let mcnf1 = MConfig::builder().load(mcv).secret("TACOS").try_build();

    // Retrieve a key and print
    println!("{:?}", mcnf.get("Hello").unwrap());

    // Retrieve a key from the duplicated one
    println!("{:?}", mcnf1.unwrap()["Hello"].as_ref().unwrap());

    // Demonstrate the iterator function
    for e in mcnf.iter() {
        println!("{:?}", e);
    }
}


/*
The C# version interface includes the following methods:

        int Count { get; }

        string? this[string key] { get; set; }

        string? Get(string key);

        void Add(string key, string? value);

        void Remove(string key);

        bool ContainsKey(string key);

        void Save();

        void SetSecret(string secret);

*/

/* the file format is simple
8,192 bytes long (by default)
header consisting of the magic bytes MCONF (0x4d, 0x43, 0x4f, 0x4e, 0x46) followed by a reserved byte for versioning
key length byte, key (UTF-8 byte string) (zero length indicates EOF, the rest is filled with random padding)
value length byte, value (Null if length is 0)

I.e.:

4d 43 4f 4e 46 vv
ll xx xx xx xx xx ... mm yy yy yy yy yy ...

first five are magic bytes
v = version byte
x = key, y = value
l = length of key in bytes, m = length of value in bytes
pattern repeats
remainder of space is padded with random bytes
 */
