#![feature(concat_idents)]
#![feature(proc_macro_hygiene)]

mod masterhand;
mod dharkon;
mod crazyhand;
mod galeem;
mod marx;

#[skyline::main(name = "playable_bosses")] 
 pub fn main() {
       galeem::install();
       dharkon::install();
       masterhand::install();
       crazyhand::install();
       marx::install();
    }
