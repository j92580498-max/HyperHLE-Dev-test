#include <stdint.h>
#include <stddef.h>

typedef struct TapHLEIOSGameMetadata TapHLEIOSGameMetadata;

TapHLEIOSGameMetadata *taphle_ios_game_metadata_create(const char *path);
const char *taphle_ios_game_metadata_display_name(const TapHLEIOSGameMetadata *metadata);
const char *taphle_ios_game_metadata_bundle_identifier(const TapHLEIOSGameMetadata *metadata);
uint32_t taphle_ios_game_metadata_orientation_capabilities(const TapHLEIOSGameMetadata *metadata);
const uint8_t *taphle_ios_game_metadata_icon_rgba(const TapHLEIOSGameMetadata *metadata);
uint32_t taphle_ios_game_metadata_icon_width(const TapHLEIOSGameMetadata *metadata);
uint32_t taphle_ios_game_metadata_icon_height(const TapHLEIOSGameMetadata *metadata);
void taphle_ios_game_metadata_free(TapHLEIOSGameMetadata *metadata);

int32_t taphle_ios_launch_game(
    const char *path,
    int32_t scale_hack,
    int32_t orientation,
    int32_t network_access,
    int32_t analog_stick_tilt_controls
);

void taphle_ios_request_exit(void);
float taphle_ios_current_fps(void);
