(module
  (import "host" "callback" (func $callback))

  (func (export "init")
    call $callback
  )
)
