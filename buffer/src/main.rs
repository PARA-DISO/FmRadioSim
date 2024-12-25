use buffer::FixedLenBuffer;
fn main() {
    let mut buffer = FixedLenBuffer::new(4, 4).unwrap();
    let mut dst = [0.; 4];
    buffer.enqueue(&[0., 1., 2., 3.]);
    // println!("{}", buffer.get_pos());
    buffer.enqueue(&[4., 5., 6., 7.]);
    // println!("{}", buffer.get_pos());
    buffer.dequeue(&mut dst);
    println!("{:?}", dst);
    // println!("{}", buffer.get_pos());
    buffer.enqueue(&[8., 9., 0., 1.]);
    buffer.dequeue(&mut dst);
    println!("{:?}", dst);
    // println!("{}", buffer.get_pos());
    buffer.enqueue(&[2., 3., 4., 5.]);
    // println!("{}", buffer.get_pos());
    buffer.enqueue(&[6., 7., 8., 9.]);
    // println!("{}", buffer.get_pos());
    buffer.dequeue(&mut dst);
    println!("{:?}", dst);
    // println!("{}", buffer.get_pos());
}
