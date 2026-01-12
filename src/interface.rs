///! IGNORE FILE!
///! NOT CONNECTED TO ANYTHING

//TODO struct interface:
pub enum Interface {
    Single(Input),
    Dual(Input, Input)
}

pub enum Input {
    Button(Box<dyn Fn(&mut Context)>),
    //Wheel(Box<dyn Fn(&mut Context, u32)>),
    DPad{
        up: Action, 
        down: Box<dyn Fn(&mut Context)>, 
        left: Box<dyn Fn(&mut Context)>, 
        right: Box<dyn Fn(&mut Context)>, 
    }
}
// single input d pad:
//  on mobile: displays d pad bottom right for up down left and right.
//  on desktop: maps both awsd arrows to up down left right


// duel and single
