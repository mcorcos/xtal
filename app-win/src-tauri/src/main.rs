// Sin la consola atrás de la ventana en Windows.
//
// Un binario de Rust es una app de consola por default: sin esto, abrir Xtal levanta
// una ventana negra de cmd al lado de la app, que queda ahí todo el tiempo. El
// `debug_assertions` la deja en los builds de desarrollo justamente para poder ver los
// `println!`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    xtal_app_lib::run()
}
