use exfat::directory::Item;
use exfat::ExFat;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[test]
fn read_image() {
    // Open the image.
    let image: PathBuf = ["tests", "exfat.img"].iter().collect();
    let image = File::open(image).expect("cannot open exfat.img");

    // Open exFAT.
    let exfat = ExFat::open(image).expect("cannot open exFAT");

    // Check image properties.
    assert_eq!(Some("Test image"), exfat.volume_label());

    // Check items in the root of image.
    let items: Vec<Item<File>> = exfat.into_iter().collect();

    assert_eq!(2, items.len());

    for i in items {
        match i {
            Item::Directory(d) => {
                // Check directory properties.
                assert_eq!("dir1", d.name());

                // Check items.
                let mut items = d.open().expect("cannot open dir1");

                assert_eq!(1, items.len());

                match items.remove(0) {
                    Item::Directory(_) => panic!("unexpected item in dir1"),
                    Item::File(mut f) => {
                        // Check file properties.
                        assert_eq!("file2", f.name());
                        assert_eq!(13, f.len());

                        // Check file content.
                        let mut c = String::new();
                        let r = f.open().expect("cannot open file2");
                        let mut r = r.expect("file2 should not be empty");

                        r.read_to_string(&mut c).expect("cannot read file2");

                        assert_eq!("Test file 2.\n", c);
                    }
                };
            }
            Item::File(mut f) => {
                // Check file properties.
                assert_eq!("file1", f.name());
                assert_eq!(13, f.len());

                // Check file content.
                let mut c = String::new();
                let r = f.open().expect("cannot open file1");
                let mut r = r.expect("file1 should not be empty");

                r.read_to_string(&mut c).expect("cannot read file1");

                assert_eq!("Test file 1.\n", c);
            }
        }
    }
}
