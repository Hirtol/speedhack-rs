fn main() {
    forward_dll::forward_dll("C:\\Windows\\System32\\version.dll").unwrap();
    forward_dll::forward_dll("C:\\Windows\\System32\\bcrypt.dll").unwrap();
    forward_dll::forward_dll("C:\\Windows\\System32\\hid.dll").unwrap();
}
