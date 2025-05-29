#include <stdio.h>
#include <libusb-1.0/libusb.h>
#include <glib.h>

const unsigned char *MAGIC = "TUL0";

int transfer(libusb_device_handle *dev, uint8_t endpoint, unsigned char *data, int data_length, int timeout) {
    int transferred;
    libusb_bulk_transfer(dev, endpoint, data, data_length, &transferred, timeout);
    return transferred;
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
                g_info("INFO: Found switch");
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
                        if (endpoint->bEndpointAddress == LIBUSB_ENDPOINT_IN) {
                            g_info("Found IN endpoint");
                            *in_ep = endpoint->bEndpointAddress;
                        } else {
                            g_info("Found OUT endpoint");
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

// Envía el header y los nombres de los archivos válidos
void send_roms(libusb_device_handle *handle, uint8_t out_ep, char *roms[], int length) {
    char *roms_list[length];
    int roms_len = 0;
    int valid_count = validate_roms(roms, length, roms_list, &roms_len);

    g_debug("roms_len: %d", roms_len);

    // Enviar header (ejemplo: MAGIC, tamaño, padding)
    unsigned char bytes_roms_length[sizeof(int)];
    unsigned char padding[8] = {0};
    memcpy(bytes_roms_length, &roms_len, sizeof(int));
    transfer(handle, out_ep, (unsigned char *)MAGIC, strlen((const char *)MAGIC), 1000);
    transfer(handle, out_ep, bytes_roms_length, sizeof(int), 1000);
    transfer(handle, out_ep, padding, sizeof(padding), 1000);

    // Enviar los nombres de los archivos uno por uno
    for (int i = 0; i < valid_count; i++) {
        transfer(handle, out_ep, (unsigned char *)roms_list[i], strlen(roms_list[i]), 1000);
        free(roms_list[i]);
    }
}

void poll_commands(libusb_device *_switch, uint8_t in_ep, uint8_t out_ep, int interfaceNum) {
    libusb_device_handle *_switch_handle;
    int length;
    int r;

    char *roms[] = {"./gta_sa.nsp", "./undertale.nsp"};

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

    send_roms(_switch_handle, out_ep, roms, 2);

    unsigned char data[0x20];
    while (1) {
        r = libusb_bulk_transfer(_switch_handle, in_ep, data, 0x20, &length, 0);
        if (r == 0) {
            printf("INFO: Read %d bytes", length);
            for (int i=0; i<length; i++) {
                printf("%02x ", data[i]);
            }
            printf("\n");
            break;
        } else {};
    }

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
    poll_commands(_switch, in_ep, out_ep, interfaceNum);

    libusb_exit(ctx);
    return 0;
}

int main() {
    return test();
}