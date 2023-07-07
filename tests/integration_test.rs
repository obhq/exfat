use exfat::directory::Item;
use exfat::image::Image;
use exfat::timestamp::Timestamp;
use exfat::Root;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn check_timestamp(
    ts: &Timestamp,
    day: u8,
    month: u8,
    year: u16,
    hour: u8,
    minute: u8,
    second: u8,
    utc_offset: i8,
) {
    assert_eq!(day, ts.date().day);
    assert_eq!(month, ts.date().month);
    assert_eq!(year, ts.date().year);
    assert_eq!(hour, ts.time().hour);
    assert_eq!(minute, ts.time().minute);
    assert_eq!(second, ts.time().second);
    assert_eq!(utc_offset, ts.utc_offset());
}

#[test]
fn read_image() {
    // Open the image.
    let image: PathBuf = ["tests", "exfat.img"].iter().collect();
    let image = File::open(image).expect("cannot open exfat.img");
    let image = Image::open(image).expect("cannot open exFAT image from exfat.img");

    // Open root directory.
    let root = Root::open(image).expect("cannot open the root directory");

    // Check image properties.
    assert_eq!(Some("Test image"), root.volume_label());

    // Check items in the root of image.
    let items = Vec::from_iter(root.into_iter());

    assert_eq!(2, items.len());

    for i in items {
        match i {
            Item::Directory(d) => {
                // Check directory properties.
                assert_eq!("dir1", d.name());

                // Check timestamps
                check_timestamp(d.timestamps().created(), 6, 3, 2023, 13, 2, 32, 0);
                check_timestamp(d.timestamps().modified(), 6, 3, 2023, 13, 3, 18, 0);
                check_timestamp(d.timestamps().accessed(), 6, 3, 2023, 13, 2, 32, 0);

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

                        f.read_to_string(&mut c).expect("cannot read file2");

                        assert_eq!("Test file 2.\n", c);

                        // Check timestamps
                        check_timestamp(f.timestamps().created(), 6, 3, 2023, 13, 3, 18, 0);
                        check_timestamp(f.timestamps().modified(), 6, 3, 2023, 13, 3, 18, 0);
                        check_timestamp(f.timestamps().accessed(), 6, 3, 2023, 13, 3, 18, 0);
                    }
                };
            }
            Item::File(mut f) => {
                // Check file properties.
                assert_eq!("file1", f.name());
                assert_eq!(13, f.len());

                // Check file content.
                let mut c = String::new();

                f.read_to_string(&mut c).expect("cannot read file1");

                assert_eq!("Test file 1.\n", c);

                // Check timestamps
                check_timestamp(f.timestamps().created(), 6, 3, 2023, 13, 3, 6, 0);
                check_timestamp(f.timestamps().modified(), 6, 3, 2023, 13, 3, 6, 0);
                check_timestamp(f.timestamps().accessed(), 6, 3, 2023, 13, 3, 6, 0);
            }
        }
    }
}
