mod invoke {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsValue;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;

        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke_no_args(cmd: &str) -> JsValue;

        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch, js_name = invoke)]
        async fn invoke_catch(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch, js_name = invoke)]
        async fn invoke_catch_no_args(cmd: &str) -> Result<JsValue, JsValue>;
    }

    use serde_wasm_bindgen::{from_value, to_value};

    pub async fn tauri_invoke<T: serde::de::DeserializeOwned, Args: serde::Serialize>(
        cmd: &str,
        args: &Args,
    ) -> T {
        let response = invoke(cmd, to_value(args).expect("To serialize args")).await;
        from_value(response).expect("to deserialize cmd response")
    }

    pub async fn tauri_invoke_no_args<T: serde::de::DeserializeOwned>(cmd: &str) -> T {
        let response = invoke_no_args(cmd).await;
        from_value(response).expect("to deserialize cmd response")
    }

    pub async fn tauri_invoke_catch<
        T: serde::de::DeserializeOwned,
        E: serde::de::DeserializeOwned,
        Args: serde::Serialize,
    >(
        cmd: &str,
        args: &Args,
    ) -> Result<T, E> {
        let response = invoke_catch(cmd, to_value(args).unwrap()).await;
        let parsed = match response {
            Ok(val) => Ok(from_value(val).expect("to deserialize cmd response")),
            Err(e) => Err(from_value(e).expect("to deserialize cmd response")),
        };

        parsed
    }

    pub async fn tauri_invoke_catch_no_args<
        T: serde::de::DeserializeOwned,
        E: serde::de::DeserializeOwned,
    >(
        cmd: &str,
    ) -> Result<T, E> {
        let response = invoke_catch_no_args(cmd).await;
        let parsed = match response {
            Ok(val) => Ok(from_value(val).expect("to deserialize cmd response")),
            Err(e) => Err(from_value(e).expect("to deserialize cmd response")),
        };

        parsed
    }

    #[macro_export]
    macro_rules! bind_command {
    ($name: ident () -> $ret:ty) => {
        pub async fn $name() -> $ret {
            $crate::tauri_interop::tauri_invoke_no_args(stringify!($name)).await
        }
    };

    ($name: ident () -> $ret_ok:ty, $ret_err:ty) => {
        pub async fn $name() -> Result<$ret_ok, $ret_err> {
            $crate::tauri_interop::tauri_invoke_catch_no_args(stringify!($name)).await
        }
    };

    ($name: ident ( $( $arg:ident : $arg_ty:ty ),+ ) -> $ret: ty) => {
        #[allow(unused_lifetimes)]
        pub async fn $name( $( $arg : $arg_ty ),+ ) -> $ret {
            #[derive(serde::Serialize)]
            #[allow(unused_lifetimes)]
            struct Args<'a> {
                $( $arg: $arg_ty ),+
                _lifetime: std::marker::PhantomData<&'a ()>,
            }
            let args = Args { $($arg),+ _lifetime: std::marker::PhantomData };
            $crate::tauri_interop::tauri_invoke(stringify!($name), &args).await
        }
    };

    ($name: ident ( $( $arg:ident : $arg_ty:ty ),+ ) -> $ret_ok:ty, $ret_err:ty) => {
        pub async fn $name( $( $arg : $arg_ty ),+ ) -> Result<$ret_ok, $ret_err> {
            #[derive(serde::Serialize)]
            #[serde(rename_all = "camelCase")]
            struct Args<'a> {
                $( $arg: $arg_ty, )+
                #[serde(skip)]
                _lifetime: std::marker::PhantomData<&'a ()>,
            }
            let args = Args { $($arg, )+ _lifetime: std::marker::PhantomData };
            $crate::tauri_interop::tauri_invoke_catch(stringify!($name), &args).await
        }
    };
}

    pub use bind_command;
}

pub use invoke::*;
