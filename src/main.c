#include <stdio.h>
#include <libusb-1.0/libusb.h>
#include <glib.h>
#include <gio/gio.h>
// #include <glib/giochannel.h>

enum CommandID {
    EXIT=0,
    FILE_RANGE=1,
    FILE_RANGE_PADDED=2
};

const int BUFFER_SEGMENT_DATA_SIZE = 0x100000;
const int PADDING_SIZE = 0x1000;
const unsigned char *MAGIC = "TUL0";
const int t = 1;
const unsigned char *TYPE_RESPONSE = (unsigned char *)t;

void clean_string(char *str, size_t len) {
    for (size_t i = 0; i < len; i++) {
        // Si el carácter no es imprimible ASCII estándar, termina el string
        if ((unsigned char)str[i] < 32 || (unsigned char)str[i] > 126) {
            str[i] = '\0';
            break;
        }
    }
}

int transfer(libusb_device_handle *dev, uint8_t endpoint, unsigned char *data, int data_length, int timeout) {
    int transferred;
    int r = libusb_bulk_transfer(dev, endpoint, data, data_length, &transferred, timeout);
    if (r < 0) {
        g_error("error while transferring: %s, %i", libusb_error_name(r), transferred);
        g_debug("endpoint %i, data length %i, data %x", endpoint, data_length, data);
    };
    return transferred;
}

char* to_string(unsigned char *bytes, int length) {
    char buff[length +1];
    memcpy(&buff, bytes, length);
    buff[length +1] = '\0';
    return buff;
}

libusb_device* find_switch(libusb_context *ctx) {
    libusb_device **devs;
    libusb_device *_switch = NULL;

    int len = libusb_get_device_list(ctx, &devs);
    for (int i; i<len; i++) {
        libusb_device *dev = devs[i];
        struct libusb_device_descriptor desc;
        int res = libusb_get_device_descriptor(dev, &desc);
        if (res == 0) {
            if (desc.idVendor == 0x057E && desc.idProduct == 0x3000) {
                _switch = dev;
                break;
            }
        }
    }
    libusb_free_device_list(devs, 1);

    return _switch;
}

void get_endpoints(libusb_device *_switch, uint8_t *in_ep, uint8_t *out_ep, uint8_t *interfaceNum) {
    // TODO: ver si puedo hacer que se cheque si se encontraron los endpoints
    struct libusb_device_descriptor desc;

    if (libusb_get_device_descriptor(_switch, &desc) == 0) {
        struct libusb_config_descriptor *config;

        if (libusb_get_active_config_descriptor(_switch, &config) == 0) {
            for (int i = 0; i < config->bNumInterfaces; i++) {
                const struct libusb_interface *interface = &config->interface[i];

                for (int j = 0; j < interface->num_altsetting; j++) {
                    const struct libusb_interface_descriptor *altsetting = &interface->altsetting[j];
                    *interfaceNum = altsetting->bInterfaceNumber;

                    for (int k = 0; k < altsetting->bNumEndpoints; k++) {
                        const struct libusb_endpoint_descriptor *endpoint = &altsetting->endpoint[k];
                        if (endpoint->bEndpointAddress == 0x81) {
                            *in_ep = endpoint->bEndpointAddress;
                        } else {
                            *out_ep = endpoint->bEndpointAddress;
                        }
                    }
                }
            }
            libusb_free_config_descriptor(config);
        }
    }
}

// Valida los archivos y calcula la longitud total
int validate_roms(char *roms[], int length, char *result[], int *roms_length) {
    int valid_count = 0;
    *roms_length = 0;

    for (int i = 0; i < length; i++) {
        char *file = roms[i];
        g_debug("validating %s...", file);
        const char *ext = strrchr(file, '.');
        if (!g_file_test(file, G_FILE_TEST_EXISTS) ||
            !ext || (strcmp(ext, ".nsp") != 0 && strcmp(ext, ".xci") != 0)) {
            g_warning("%s is not a valid rom", file);
            continue;
        }
        // Reservar espacio para el string con salto de línea
        size_t len = strlen(file) + 2;
        result[valid_count] = malloc(len);
        snprintf(result[valid_count], len, "%s\n", file);
        *roms_length += strlen(file) + 1;
        valid_count++;
    }
    return valid_count;
}

void send_rom_list(libusb_device_handle *handle, uint8_t out_ep, char *roms[], int length) {
    g_debug("sending rom list...");
    char *roms_list[length];
    int roms_len = 0;
    int valid_count = validate_roms(roms, length, roms_list, &roms_len);

    // Send header
    unsigned char bytes_roms_length[sizeof(int)];
    unsigned char padding[8] = {0};
    memcpy(bytes_roms_length, &roms_len, sizeof(int));
    transfer(handle, out_ep, MAGIC, strlen((const char *)MAGIC), 1000);
    transfer(handle, out_ep, bytes_roms_length, sizeof(int), 1000);
    transfer(handle, out_ep, padding, sizeof(padding), 1000);

    // Send roms names
    for (int i = 0; i < valid_count; i++) {
        transfer(handle, out_ep, (unsigned char *)roms_list[i], strlen(roms_list[i]), 1000);
        free(roms_list[i]);
    }
}

int send_file(libusb_device_handle *_switch, uint8_t in_ep, uint8_t out_ep, int _cmd_id, int padding) {
    GError *error = NULL;
    GFile *file;
    GFileInfo *info;
    GFileInputStream *stream;

    unsigned char header[0x20];
    uint64_t range_size;
    uint64_t range_offset;
    uint64_t rom_name_len;
    unsigned char first_padding[3] = {0};
    unsigned char last_padding[0xC] = {0};

    unsigned char *cmd_id = (unsigned char *)_cmd_id;
    unsigned char *encoded_range_size = (unsigned char *)range_size;

    g_debug("sending file...");
    transfer(_switch, in_ep, header, sizeof(header), 0);

    memcpy(&range_size, &header[0], 8);
    memcpy(&range_offset, &header[8], 8);
    memcpy(&rom_name_len, &header[16], 8);

    g_debug("rom_name_len %i", rom_name_len);
    unsigned char rom_name_buff[rom_name_len];
    char rom_name[rom_name_len];
    transfer(_switch, in_ep, rom_name_buff, rom_name_len, 0);
    memcpy(rom_name, rom_name_buff, rom_name_len);

    // send file response header
    g_debug("magic");
    transfer(_switch, out_ep, MAGIC, sizeof(MAGIC), 0);
    g_debug("response type");
    int x = transfer(_switch, out_ep, TYPE_RESPONSE, sizeof(TYPE_RESPONSE), 0);
    g_debug("first padding %i", x);
    transfer(_switch, out_ep, first_padding, 3, 0);
    g_debug("cmd id");
    transfer(_switch, out_ep, cmd_id, sizeof(cmd_id), 0); // check this function if segfaults
    g_debug("encoded_range_size");
    transfer(_switch, out_ep, encoded_range_size, sizeof(encoded_range_size), 0); // this one too
    g_debug("last padding");
    transfer(_switch, out_ep, last_padding, 0xC, 0);
    // FIXME: LAST
    char clean_rom[rom_name_len];
    memcpy(clean_rom, rom_name, rom_name_len);
    // clean_string(clean_rom, rom_name_len);
    
    g_debug("note: if segfaults add zero terminator to rom_name");
    g_debug("rom \"%s\" recieved from switch", clean_rom);

    g_debug("creating file object");
    file = g_file_new_for_path(clean_rom);
    if (error != NULL) {
        g_printerr("%s\n", error->message);
        g_error_free(error);
        return -1;
    }

    g_debug("creating querier");
    info = g_file_query_info(file, G_FILE_ATTRIBUTE_STANDARD_SIZE, G_FILE_QUERY_INFO_NONE, NULL, &error);
    if (error != NULL) {
        g_printerr("%s\n", error->message);
        g_error_free(error);
        g_object_unref(file);
        return -1;
    }

    g_debug("opening file for reading");
    stream = g_file_read(file, NULL, &error);
    if (error != NULL) {
        g_printerr("%s\n", error->message);
        g_error_free(error);
        g_object_unref(file);
        g_object_unref(info);
        return -1;
    }

    g_debug("reading");
    size_t current_offset = 0;
    size_t read_size = BUFFER_SEGMENT_DATA_SIZE;
    if (padding == TRUE)
        read_size -= PADDING_SIZE;
    
    unsigned char *buf = malloc(BUFFER_SEGMENT_DATA_SIZE);
    unsigned char *pad_buf = NULL;
    if (padding)
        pad_buf = calloc(PADDING_SIZE, 1);
    
    while (current_offset < range_size) {
        if (current_offset + read_size > range_size)
            read_size = range_size - current_offset;
        
        gssize bytes_read = g_input_stream_read(G_INPUT_STREAM(stream), buf, read_size, NULL, &error);
        if (padding) {
            ssize_t total_size = PADDING_SIZE + bytes_read;
            unsigned char *send_buf = malloc(total_size);
            memcpy(send_buf, pad_buf, PADDING_SIZE);
            memcpy(send_buf + PADDING_SIZE, buf, bytes_read);
            transfer(_switch, out_ep, send_buf, total_size, 0);
            free(send_buf);
        } else {
            transfer(_switch, out_ep, buf, bytes_read, 0);
        }
        current_offset += bytes_read;
    }
    // goffset rom_size = g_file_info_get_size(info);

}

void poll_commands(libusb_device *_switch, uint8_t in_ep, uint8_t out_ep, int interfaceNum) {
    libusb_device_handle *_switch_handle;
    int length;
    int r;

    char *roms[] = {"./roms/gta_sa.nsp", "./roms/undertale.nsp"};

    r = libusb_open(_switch, &_switch_handle);
    if (r != 0) {
        g_warning("Couldn't open switch device. %i", r);
        return;
    }

    r = libusb_claim_interface(_switch_handle, interfaceNum);
    if (r != 0) {
        g_warning("Couldn't claim the switch interface. %i", r);
        return;
    }

    send_rom_list(_switch_handle, out_ep, roms, 2);

    unsigned char data[0x20];
    while (1) {
        r = libusb_bulk_transfer(_switch_handle, in_ep, data, 0x20, &length, 0);
        if (r == 0) {
            uint32_t cmd_id;
            uint8_t cmd_type;
            uint64_t data_size;
            char _magic[5];
            memcpy(_magic, data, 4);
            _magic[5] = '\0';

            g_debug("successful read, magic: %s", _magic);
            if (strcmp(_magic, "TUC0") != 0) {
                g_warning("Invalid magic: %s", _magic);
                continue;
            }
            
            cmd_type = data[4];
            memcpy(&cmd_id, &data[8], sizeof(uint32_t));
            memcpy(&data_size, &data[12], sizeof(uint64_t));

            g_debug("checking command...");
            if (cmd_id == EXIT) {
                g_debug("Exit recieved");
                break;
            } else if (cmd_id == FILE_RANGE || cmd_id == FILE_RANGE_PADDED) {
                g_debug("file operation");
                if (send_file(_switch_handle, in_ep, out_ep, cmd_id, cmd_id == FILE_RANGE_PADDED ? TRUE : FALSE) == -1)
                    break;
            } else {
                g_warning("Unknown command id");
            }
            
        } else {
            g_warning("Error while trying to read from usb: %s", libusb_error_name(r));
            break;
        };
    }

    g_debug("releasing device...");
    libusb_release_interface(_switch_handle, interfaceNum);
    libusb_close(_switch_handle);
}

int test() {
    libusb_context *ctx = NULL;
    uint8_t in_ep;
    uint8_t out_ep;
    uint8_t interfaceNum;

    int r = libusb_init(&ctx);
    if (r < 0) {
        g_warning("Failed to init libusb");
        return 1;
    }

    libusb_device *_switch = find_switch(ctx);
    if (_switch == NULL) {
        g_warning("Couldn't find switch");
        return 2;
    }

    get_endpoints(_switch, &in_ep, &out_ep, &interfaceNum);
    g_debug("polling commands");
    poll_commands(_switch, in_ep, out_ep, interfaceNum);

    libusb_exit(ctx);
    return 0;
}

int main() {
    return test();
}