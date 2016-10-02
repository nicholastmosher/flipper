{-|
Module      : Flipper.Internal.GPIO
Description : Internal GPIO Module
Copyright   : George Morgan, Travis Whitaker 2016
License     : All rights reserved.
Maintainer  : travis@flipper.io
Stability   : Provisional
Portability : Windows, POSIX

-}

module Flipper.Internal.GPIO (
    DigitalPin(..)
  , AnalogPin(..)
  , Direction(..)
  , digitalDirection
  , analogDirection
  , digitalRead
  , analogRead
  , digitalWrite
  , analogWrite
  ) where

import Foreign.Marshal.Utils

import Data.Word

data DigitalPin = IO1
                | IO2
                | IO3
                | IO4
                | IO5
                | IO6
                | IO7
                | IO8
                | IO9
                | IO10
                | IO11
                | IO12
                | IO13
                | IO14
                | IO15
                | IO16
                deriving (Eq, Ord, Show)

data AnalogPin = A1
               | A2
               | A3
               | A4
               | A5
               | A6
               | A7
               | A8
               deriving (Eq, Ord, Show)

data Direction = Input
               | Output
               deriving (Eq, Ord, Show)

digPinCode :: DigitalPin -> Word8
digPinCode IO1  = 1
digPinCode IO2  = 2
digPinCode IO3  = 3
digPinCode IO4  = 4
digPinCode IO5  = 5
digPinCode IO6  = 6
digPinCode IO7  = 7
digPinCode IO8  = 8
digPinCode IO9  = 9
digPinCode IO10 = 10
digPinCode IO11 = 11
digPinCode IO12 = 12
digPinCode IO13 = 13
digPinCode IO14 = 14
digPinCode IO15 = 15
digPinCode IO16 = 16

anPinCode :: AnalogPin -> Word8
anPinCode A1 = 17
anPinCode A2 = 18
anPinCode A3 = 19
anPinCode A4 = 20
anPinCode A5 = 21
anPinCode A6 = 22
anPinCode A7 = 23
anPinCode A8 = 24

directionCode :: Direction -> Word8
directionCode Input  = 0
directionCode Output = 1

digitalDirection :: DigitalPin -> Direction -> IO ()
digitalDirection p d = c_gpio_set_direction (digPinCode p) (directionCode d)

analogDirection :: AnalogPin -> Direction -> IO ()
analogDirection p d = c_gpio_set_direction (anPinCode p) (directionCode d)

digitalRead :: DigitalPin -> IO Bool
digitalRead p = toBool <$> c_gpio_read (digPinCode p)

analogRead :: AnalogPin -> IO Word16
analogRead p = c_gpio_read (anPinCode p)

digitalWrite :: DigitalPin -> Bool -> IO ()
digitalWrite p v = c_gpio_write (digPinCode p) (fromBool v)

analogWrite :: AnalogPin -> Word16 -> IO ()
analogWrite p = c_gpio_write (anPinCode p)

foreign import ccall safe "flipper/gpio.h gpio_set_direction"
    c_gpio_set_direction :: Word8 -> Word8 -> IO ()

foreign import ccall safe "flipper/gpio.h gpio_write"
    c_gpio_write :: Word8 -> Word16 -> IO ()

foreign import ccall safe "flipper/gpio.h gpio_read"
    c_gpio_read :: Word8 -> IO Word16
