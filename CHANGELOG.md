# Changelog

## Unreleased for 0.3.0-alpha.1

- Show a landscape game's launch image the way it was drawn. tapHLE looked only
  for `Default.png` — the portrait image such a game never actually shows — and
  then turned it on its side to fill the window, so games opened on a squashed,
  sideways splash. Games that ship a left- and a right-handed image, such as
  Baby Monkey, were also handed the one for the opposite orientation and came up
  upside down.
- Stop Unity games aborting when their garbage collector restores access to
  memory it had released. tapHLE has no page protection to apply, so refusing
  the request described tapHLE rather than the game's memory, and Mono shut the
  app down over it.
- Stop claiming OpenGL ES support for vertex array objects, which tapHLE does
  not have. Games that believed the claim drew whole scenes with another
  object's geometry; Cubed Rally Redline's track and terrain were invisible.
- Say which file a failed `open()` was looking for. The warning previously
  printed only the address the filename happened to sit at, so a game quietly
  missing a level or a save gave no clue what it wanted.
- Let older JSON parsers reserve mutable byte storage and access NSObject's
  root ivar offset, advancing Cubed Rally Redline's review and results paths.
- Let OpenGL ES 1 games use non-power-of-two textures and render targets on
  Windows, fixing games that otherwise disable their requested display mode.
- Keep running when a game builds a Foundation collection by hand instead of
  through `+alloc`. JSONKit, which many games bundle to read JSON, does this,
  and it previously ended the app the moment a parsed result came back.
- Stop Unity games dying when their garbage collector recycles memory. Mono's
  collector releases and reclaims its own heap by re-mapping it in place, and
  tapHLE refused, so the game shut itself down — in Cubed Rally Redline, while
  loading a second race.
- Rebrand the fork as tapHLE across the executable, crates, runtime resources,
  internal symbols, tests, and packaging.
- Define Windows and modern iOS game compatibility as the product focus, with
  macOS retained for development and iOS builds and Android outside the active
  support scope.
- Add an experimental native host for modern iOS with a SwiftUI game library,
  app importing, device orientation, save persistence, JIT handoff, in-game
  exit control, and iOS OpenGL ES presentation.
- Add AI-agent contribution guidance, provenance rules, upstream quarantine
  instructions, game-focused issue forms, and repository policy checks.
- Add a simple self-service guide for people using coding agents to improve a
  game they care about.
- Move the current compatibility record to the live tapHLEdb service while
  retaining legacy JSON records for migration and offline validation.
- Add exact Archive.org compatibility provenance and verified three-star
  in-game milestones for Ricky, Baby Monkey, Cops & Robbers, and SPYmouse HD.
- Add MP3 streaming support, AudioQueue OpenAL source cleanup, clean
  secondary-thread shutdown, and state-aware EAGL frame capture for
  agent-driven Windows testing.
- Start hundreds of apps that previously died before showing anything. Support
  was chosen by surveying where 1501 apps stop and fixing the most common
  causes: Objective-C autorelease-pool and exception-handler entry points,
  runtime class creation, integer-division and C++ allocation builtins, and a
  large set of Foundation, UIKit, Core Animation, Media Player and Audio
  Session constants that apps read during startup.
- Stop treating ordinary runtime conditions as fatal. Retaining a constant
  string, asking for a class that does not exist, an out-of-range array index,
  a mapping the allocator cannot satisfy, and messaging an object that was
  released too many times are all defined or recoverable on a device, and each
  previously ended the app.
- Add UITabBarController, UIToolbar, UIProgressView, UIPageControl,
  NSURLProtocol, NSCondition and NSUbiquitousKeyValueStore, and let nibs decode
  bar items, colours in more colour spaces, and navigation and scroll view
  properties.
- Verified three-star in-game milestones for Tap Tap Revenge 2, Omium,
  Parachute Panic HD and Scoops, and a two-star start for JellyCar.
- Start more apps still: glGetIntegerv now answers for every parameter rather
  than only integer ones, and Objective-C block retention, run-time class
  creation, Core Graphics colour components, Foundation allocation zones and
  extended-attribute calls all work.
- Stop ending the app over ordinary input. A locale identifier carrying a
  script or a numeric region, an OpenGL ES parameter name the backend does not
  know, a nib that produces no objects or sets no view, a message whose
  declared argument types disagree with the receiver's, and an application
  started without a delegate class are all things a device accepts, and each
  one previously aborted at launch.
- Keep going through more ordinary input still. Scheduling a timer that is
  already scheduled, asking for virtual-memory statistics with room to spare,
  reading a localized string from a bundle other than the main one, messaging
  an object whose class is no longer there, loading data that is not valid text
  in the encoding it was declared to be in, loading an image in a format tapHLE
  cannot decode, `dlopen` of a library that is not present, asking a Core
  Foundation array for a starting capacity, enabling or hinting an OpenGL ES
  capability the backend does not model, setting an audio unit property on the
  input bus, transforming by a matrix that has picked up a NaN, and running
  `sscanf` off the end of its input are all things a device takes in its
  stride, and each one previously ended the app where it happened. The same
  reading of the survey also cleared an `Info.plist` that writes its supported
  orientations as a string or a dictionary rather than an array, a table view
  whose delegate sets no row height, and a `CFPreferences` call that names the
  app's own bundle identifier rather than the current-application constant. Of
  the 72 apps a survey of 1501 found stopping at these points, all 72 now get
  past them.
- Read what apps were already asking for. NSScanner scans hexadecimal numbers,
  `NSURL` accepts a `file:` URL wherever a URL string is accepted and reports
  the path of a URL that has a scheme and host in front of it, `.strings` files
  load whether they are UTF-8 or UTF-16 and whether or not their keys are
  quoted, a byte order mark is consumed rather than left at the front of the
  text, and `sscanf` reports the end of its input as `EOF` rather than as no
  match.
- Run games built with Unity. The engine now starts, loads its Mono runtime and
  scenes, and reaches gameplay. This needed the 64-bit integer-to-float
  conversion helpers a managed runtime uses for ordinary arithmetic, the UTF-16
  and mutable entry points of `CFString` and `CFData`, detaching a renderbuffer
  from its drawable when a surface is rebuilt, and a Core Motion that describes
  a device without a gyroscope instead of stopping when asked for one.
- Stop freezing when an app asks for memory at a particular address. Where the
  requested address could not be used, only one of the two reasons it might be
  unusable was recovered from, so a mapping could be refused with most of the
  address space free. An allocator that retries — which is what a managed
  runtime does — then never got its memory, and the game sat on its last frame
  looking as though it were still running.
- Cubed Rally Redline reaches its menus, car select and a race, and now draws
  them. It renders landscape directly rather than letting tapHLE rotate the
  screen for it, so it was being rotated twice and the finished frame arrived
  as flat horizontal bands; it now launches in landscape-native mode. Racing,
  crashing, reading the results and starting another race all work.

### Known limitations

- Throwing an Objective-C or C++ exception is still unimplemented and reports
  itself clearly; registering the handlers, which is what almost every app
  actually needed, works.
- UITabBarController and UIToolbar hold and report their contents but are not
  drawn, so an app relying on the tab bar to change tabs stays on the first tab.
- Zoom, pattern colours and content gravity are stored and reported back rather
  than applied.
- Audio queues cannot render offline, which is how Unity decodes compressed
  clips, so those sounds are missing. The call now fails the way a device fails
  it when the codec is busy, which apps are written to survive, rather than
  ending the game.
- Core Motion reports no accelerometer data. Apps reading tilt through
  `UIAccelerometer` are unaffected; those reading it through Core Motion see no
  movement, so tilt steering does not work in them.

Earlier release history belongs to the upstream touchHLE project and can be
found in its repository history.
