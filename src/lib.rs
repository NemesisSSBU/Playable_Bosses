#![feature(concat_idents)]
#![feature(proc_macro_hygiene)]

mod masterhand;
mod crazyhand;
mod dharkon;
mod galeem;
mod marx;
mod playable_masterhand;
mod dracula;
mod rathalos;
mod galleom;
mod ganon;
mod gigabowser;
mod tabuu;
//mod debug_masterhand;
//mod waluigi;
//mod killdebug;

#[skyline::main(name = "comp_playable_bosses")] 
 pub fn main() {
       masterhand::install();
       crazyhand::install();
       galeem::install();
       dharkon::install();
       marx::install();
       playable_masterhand::install();
       rathalos::install();
       dracula::install();
       galleom::install();
       ganon::install();
       gigabowser::install();
       tabuu::install();
       //debug_masterhand::install();
       //waluigi::install();
       //killdebug::install();
    }
