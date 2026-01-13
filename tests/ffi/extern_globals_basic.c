int g_counter = 7;

int c_read_counter(void) { return g_counter; }

void c_write_counter(int value) { g_counter = value; }

void extern_global_anchor(void) {}
