

pub trait Driver {
    fn name(&self) -> &'static str;
}

pub trait Console: Driver {
    fn put_char(&mut self, value: u8);
    fn get_char(&mut self) -> Option<u8>;
    fn wait_for_char(&mut self) -> u8 {
        loop {
            if let Some(ch) = self.get_char() {
                return ch;
            }
        }
    }
}
