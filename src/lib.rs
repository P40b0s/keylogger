mod hook;
mod keys;
pub use keys::VirtualKey;
pub use hook::KeysWatcher;





#[cfg(test)]
mod tests
{
    use std::{sync::Arc, time::Duration};
    use crate::{hook::KeysWatcher, keys::VirtualKey};

    #[tokio::test]
    async fn run_test()
    {
        let _ = logger::StructLogger::new_default();
        let state = Arc::new(String::from("TEST_STATE"));
        let mut key_watcher = KeysWatcher::new();
        key_watcher
        .register(&[VirtualKey::LeftCtrl, VirtualKey::LeftAlt], callback_1).await
        .register(&[VirtualKey::F5, VirtualKey::MouseLeftClick], callback_2).await
        .register_with_state(&[VirtualKey::LeftCtrl, VirtualKey::RightArrow], state, callback_3).await
        .watch();
        loop 
        {
            tokio::time::sleep(Duration::from_millis(5000)).await;
        }
    }
    async fn callback_1()
    {
        logger::info!("left control + left alt!");
    }
    async fn callback_2()
    {
        logger::info!("F5 + mouse left click");
    }
    async fn callback_3(state: Arc<String>)
    {
        logger::info!("{}", ["F5 + mouse left click", &state].concat());
    }
}