Since 1.2
=========

* [#1044](https://github.com/tock/tock/pull/1044) creates a `Kernel` struct
  with a method for the kernel's main loop, instead of a global function in
  the kernel's base module. Board configurations (i.e. each board's
  `main.rs`), as a result needs to instantiate a statically allocate this new
  struct.  Arguments to the main loop haven't changed:

  ```rust
  let board_kernel = static_init!(kernel::Kernel, kernel::Kernel::new());

  board_kernel.kernel_loop(&hail, &mut chip, &mut PROCESSES, Some(&hail.ipc));
  ```


* [#1032](https://github.com/tock/tock/pull/1032) updates the ADC HIL to
  explicitly specify the resolution of the sample and to clarify that samples
  are always left-aligned in the `u16` buffer. Previously, the only ADC
  implementation happened to be 12 bits and left-aligned, which callers only
  assumed. It also added a method to (if possible) report the reference
  voltage, which can be used to convert raw ADC samples to absolute voltages.
  Implementers of the ADC HIL must implement two new methods:

  ```rust
  /// Function to ask the ADC how many bits of resolution are in the samples
  /// it is returning.
  fn get_resolution_bits(&self) -> usize;

  /// Function to ask the ADC what reference voltage it used when taking the
  /// samples. This allows the user of this interface to calculate an actual
  /// voltage from the ADC reading.
  ///
  /// The returned reference voltage is in millivolts, or `None` if unknown.
  fn get_voltage_reference_mv(&self) -> Option<usize>;
  ```

* UART HIL Refinements: This release saw several updates to the UART HIL,
  summarized in the [UART HIL tracking issue](https://github.com/tock/tock/issues/1072).

  - [#1073](https://github.com/tock/tock/pull/1073) removes `initialize` from
    the UART HIL. Implementations will need to disentangle board-specific
    initialization code, such as enabling the peripheral or assigning pins,
    from UART configuration code, such as baud rate or parity. Initialization
    is no longer part of the UART HIL and should be performed by the top-level
    board before passing the UART object to any other code. UART configuration
    is now controlled by the new `configure` HIL method:

    ```rust
    /// Configure UART
    ///
    /// Returns SUCCESS, or
    ///
    /// - EOFF: The underlying hardware is currently not available, perhaps
    ///         because it has not been initialized or in the case of a shared
    ///         hardware USART controller because it is set up for SPI.
    /// - EINVAL: Impossible parameters (e.g. a `baud_rate` of 0)
    /// - ENOSUPPORT: The underlying UART cannot satisfy this configuration.
    fn configure(&self, params: UARTParameters) -> ReturnCode;
    ```
