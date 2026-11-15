//! Platform-independent Gomoku core: rules, camera, CPU, game flow.
//! No I/O, no threads — shared by the terminal and web frontends.

pub mod ai;
pub mod app;
pub mod board;
pub mod camera;
