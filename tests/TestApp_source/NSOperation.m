#import "system_headers.h"

static int operation_value;

@interface TestOperation : NSOperation
@end

@implementation TestOperation
- (void)main {
  if (![self isExecuting])
    operation_value = -100;
  else
    operation_value = 1;
}
@end

@interface InvocationTarget : NSObject
- (void)record:(id)object;
@end

@implementation InvocationTarget
- (void)record:(id)object {
  if (object == self)
    operation_value = 2;
}
@end

int test_NSOperation(void) {
  TestOperation *operation = [TestOperation new];
  if (![operation isReady] || [operation isExecuting] ||
      [operation isFinished] || [operation isCancelled])
    return -1;

  NSOperationQueue *queue = [NSOperationQueue new];
  [queue addOperation:operation];
  if (operation_value != 1 || [operation isExecuting] ||
      ![operation isFinished])
    return -2;
  if ([queue operationCount] != 0 || [[queue operations] count] != 0)
    return -3;

  InvocationTarget *target = [InvocationTarget new];
  NSInvocationOperation *invocation =
      [[NSInvocationOperation alloc] initWithTarget:target
                                          selector:@selector(record:)
                                            object:target];
  [queue addOperation:invocation];
  if (operation_value != 2 || ![invocation isFinished])
    return -4;

  TestOperation *cancelled = [TestOperation new];
  [cancelled cancel];
  [queue addOperation:cancelled];
  if (![cancelled isCancelled] || ![cancelled isFinished] ||
      operation_value != 2)
    return -5;

  [cancelled release];
  [invocation release];
  [target release];
  [queue release];
  [operation release];
  return 0;
}
