//
//  LFDevice.m
//  Flipper
//
//  Created by George Morgan on 12/27/16.
//  Copyright © 2016 Flipper. All rights reserved.
//

#import "LFDevice.h"
#include <flipper/libflipper.h>

@implementation LFDevice

- (id) initWithName:(NSString *)name {
    if (self == [super init]) {
        flipper_attach_usb([name UTF8String]);
        return self;
    }
    return NULL;
}

@end
