
use std::ffi::CStr;
use std::ptr;

pub fn control_printer() {
    unsafe {
        let mut num_dests: i32 = 0;
        let dests = cups::cupsGetDests(&mut num_dests);

        println!("Found {} printers:", num_dests);

        for i in 0..num_dests {
            let dest = *dests.offset(i as isize);

            let name = CStr::from_ptr(dest.name);
            let name_str = name.to_string_lossy();

            println!(" - {}", name_str);
        }

        cups::cupsFreeDests(num_dests, dests);
    }
}
