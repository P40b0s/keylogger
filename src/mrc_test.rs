use std::{any::Any, collections::HashSet, future::Future, pin::Pin, sync::{atomic::AtomicBool, Arc, RwLock}, time::Duration};
use std::fmt::Debug;

type AsyncFn = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
//убрал arc так как почему то при даункасте получается 2 ссылки... но как теперь использовать клонирование? этот тип никак не склонировать...
type Argument = Box<dyn Any + Send + Sync>;
type AsyncArgFn = Arc<dyn Fn(Argument) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub trait IsArc {}
impl<T> IsArc for Arc<T>{}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum VirtualKey 
{
    Backspace,
    Tab,
    Enter,
}
#[derive(Clone)]
struct HotKeyCallback
{
    keys: HashSet<VirtualKey>,
    func:  HotKeyCallbackEnum,
}

enum HotKeyCallbackEnum
{
    WithArg(AsyncArgFn, Argument),
    WithoutArg(AsyncFn)
}
impl Clone for HotKeyCallbackEnum
{
    fn clone(&self) -> Self 
    {
        match self
        {
            HotKeyCallbackEnum::WithArg(v, a) => HotKeyCallbackEnum::WithArg(Arc::clone(v), a),
            HotKeyCallbackEnum::WithoutArg(v) => HotKeyCallbackEnum::WithoutArg(Arc::clone(v))
        }
    }
}

pub struct KeysWatcher
{
    callbacks: Arc<RwLock<Vec<HotKeyCallback>>>,
    kill: Arc<AtomicBool>
}
impl KeysWatcher
{
    pub fn new() -> Self
    {
        Self
        {
            callbacks: Arc::new(RwLock::new(Vec::new())),
            kill: Arc::new(AtomicBool::new(false))
        }
    }
    pub fn register<F, Fut>(&mut self, keys: &[VirtualKey], f: F) -> &mut Self
    where 
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let hk = HotKeyCallback
        {
            keys: HashSet::from_iter(keys.to_owned().into_iter()),
            func: HotKeyCallbackEnum::WithoutArg(Arc::new( move || Box::pin(f())))
        };
        let mut guard = self.callbacks.write().unwrap();
        guard.push(hk);
        drop(guard);
        self
    }
    pub fn register_with_state<F, Fut, Arg>(&mut self, keys: &[VirtualKey], s: Arg, f: F) -> &mut Self
    where 
        F: Fn(Arg) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        Arg: IsArc + Send + Sync + 'static + Clone + Debug
    {
        let callback = Arc::new(move |arg: Argument|
        {
            //почему 2 то?
            logger::debug!("strong counts: {}", Arc::strong_count(&arg));
            let arg = Arc::try_unwrap(arg).unwrap();
            let arg = arg.downcast::<Arg>().unwrap();
            Box::pin(f(*arg)) as Pin<Box<(dyn Future<Output = ()> + Send + 'static)>>
        });
        
        let hk = HotKeyCallback
        {
            keys: HashSet::from_iter(keys.to_owned().into_iter()),
            func: HotKeyCallbackEnum::WithArg(callback, Arc::new(Box::new(s)))
        };
        let mut guard = self.callbacks.write().unwrap();
        guard.push(hk);
        drop(guard);
        self
    }
    pub fn watch(&self)
    {
        let callbacks = self.callbacks.clone();
        let mut cb_guard  = callbacks.write().unwrap();
        let callbacks = std::mem::replace(&mut *cb_guard, Vec::<HotKeyCallback>::new());
        drop(cb_guard);
        tokio::spawn(async move
        {
            for callback in callbacks.iter()
            {
                let funcs = callback.func.clone();
                match funcs
                {
                    HotKeyCallbackEnum::WithoutArg(f) =>
                    {
                        let s = tokio::spawn(async move 
                        {
                            f().await;
                        });
                        let r = s.await;
                        logger::debug!("{:?}", r);
                    },
                    HotKeyCallbackEnum::WithArg(f, a) =>
                    {
                        let s = tokio::spawn(async move 
                        {
                            f(a).await;
                        });
                        let r = s.await;
                        logger::debug!("{:?}", r);
                    }
                }
                logger::debug!("keys fire");
            }
        });
    }
}

#[tokio::main]
async fn main()
{
    let _ = logger::StructLogger::new_default();
    let state = Arc::new(String::from("TEST_STATE"));
    let mut key_watcher = KeysWatcher::new();
    key_watcher
    .register(&[VirtualKey::Backspace, VirtualKey::Enter], callback_1)
    .register(&[VirtualKey::Backspace, VirtualKey::Tab], callback_2)
    .register_with_state(&[VirtualKey::Tab, VirtualKey::Enter], state, callback_3)
    .watch();
    loop 
    {
        tokio::time::sleep(Duration::from_millis(5000)).await;
    }
}

async fn callback_1()
{
    logger::info!("callback_1");
}
async fn callback_2()
{
    logger::info!("callback_2");
}
async fn callback_3(state: Arc<String>)
{
    logger::info!("{}", ["callback_3 + state: ", &state].concat());
}
#[cfg(test)]
mod tests
{
    use std::{sync::Arc, time::Duration};

    use super::{KeysWatcher, VirtualKey};
    
    #[tokio::test]
    async fn run_test()
    {
        let _ = logger::StructLogger::new_default();
        let state = Arc::new(String::from("TEST_STATE"));
        let mut key_watcher = KeysWatcher::new();
        key_watcher
        .register(&[VirtualKey::Backspace, VirtualKey::Enter], callback_1)
        .register(&[VirtualKey::Backspace, VirtualKey::Tab], callback_2)
        .register_with_state(&[VirtualKey::Tab, VirtualKey::Enter], state, callback_3)
        .watch();
        
        tokio::time::sleep(Duration::from_millis(1000)).await;
        
    }
    async fn callback_1()
    {
        logger::info!("callback_1");
    }
    async fn callback_2()
    {
        logger::info!("callback_2");
    }
    async fn callback_3(state: Arc<String>)
    {
        logger::info!("{}", ["callback_3 + state: ", &state].concat());
    }
}