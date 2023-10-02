use safe_drive::{
    context::Context, error::DynError, logger::Logger, pr_info, topic::publisher::Publisher,
};
use futures::stream::StreamExt;
use gpio_cdev::{Chip, AsyncLineEventHandle,LineRequestFlags, EventRequestFlags, EventType, LineEvent};

#[repr(u8)]
#[derive(Debug)]
enum AxlePosition{
    FRONT,
    MIDDLE,
    REAR
}

#[tokio::main]
async fn main() -> Result<(), DynError>{
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let front_pin_number = 135;
    let middle_pin_number = 134;
    let rear_pin_number = 133;

    let line_front = chip.get_line(front_pin_number)?;
    let line_middle = chip.get_line(middle_pin_number)?;
    let line_rear = chip.get_line(rear_pin_number)?;
    
    let mut events_front = AsyncLineEventHandle::new(line_front.events(
        LineRequestFlags::INPUT, 
        EventRequestFlags::BOTH_EDGES, 
    "frontevents",
    )?)?;
    let mut events_middle = AsyncLineEventHandle::new(line_middle.events(
        LineRequestFlags::INPUT, 
        EventRequestFlags::BOTH_EDGES, 
    "middleevents",
    )?)?;
    let mut events_rear = AsyncLineEventHandle::new(line_rear.events(
        LineRequestFlags::INPUT, 
        EventRequestFlags::BOTH_EDGES, 
    "rearevents",
    )?)?;

    let ctx = Context::new()?;
    let node = ctx.create_node("block_detector", None, Default::default())?;

    let publisher = node.create_publisher::<drobo_interfaces::msg::SolenoidStateMsg>("solenoid_order", None)?;

    loop{
        tokio::select! {
            Some(event) = events_front.next() => solenoid_publisher(&publisher, AxlePosition::FRONT, event?).await,
            Some(event) = events_middle.next() => solenoid_publisher(&publisher, AxlePosition::MIDDLE, event?).await,
            Some(event) = events_rear.next() => solenoid_publisher(&publisher, AxlePosition::REAR, event?).await,
        }
    }
}

async fn solenoid_publisher(publisher: &Publisher<drobo_interfaces::msg::SolenoidStateMsg>, axle_position: AxlePosition, event: LineEvent){
    let logger = Logger::new("block_detector");
    let mut msg = drobo_interfaces::msg::SolenoidStateMsg::new().unwrap();
    msg.axle_position = axle_position as u8;
    match event.event_type(){
        EventType::RisingEdge => {
            pr_info!(logger, "{}を上昇", msg.axle_position);
            msg.state = true;
        }
        EventType::FallingEdge => {
            let block_over_time = 2;
            let _ = tokio::time::sleep(tokio::time::Duration::from_secs(block_over_time)).await;
            pr_info!(logger, "{}を下降", msg.axle_position);
            msg.state = false;
        }
    }
    let _ = publisher.send(&msg);
}