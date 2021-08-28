#![feature(concat_idents)]
#![feature(proc_macro_hygiene)]

mod masterhand;
mod dharkon;
mod crazyhand;
mod galeem;
mod marx;
mod playable_masterhand;
mod dracula;
mod rathalos;
//mod waluigi;
//mod killdebug;

#[skyline::main(name = "playable_bosses")] 
 pub fn main() {
       galeem::install();
       dharkon::install();
       masterhand::install();
       crazyhand::install();
       marx::install();
       playable_masterhand::install();
       rathalos::install();
       //waluigi::install();
       dracula::install();
       //killdebug::install();
    }
