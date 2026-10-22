/* Minimal test-only StatusNotifier host on an isolated dbus-run-session bus. */
#include <gio/gio.h>
#include <stdio.h>

static GMainLoop *loop;
static GDBusNodeInfo *info;
static const char *xml =
  "<node><interface name='org.kde.StatusNotifierWatcher'>"
  "<method name='RegisterStatusNotifierItem'><arg type='s' direction='in'/></method>"
  "<method name='RegisterStatusNotifierHost'><arg type='s' direction='in'/></method>"
  "<property name='RegisteredStatusNotifierItems' type='as' access='read'/>"
  "<property name='IsStatusNotifierHostRegistered' type='b' access='read'/>"
  "<property name='ProtocolVersion' type='i' access='read'/>"
  "<signal name='StatusNotifierItemRegistered'><arg type='s'/></signal>"
  "<signal name='StatusNotifierItemUnregistered'><arg type='s'/></signal>"
  "<signal name='StatusNotifierHostRegistered'/>"
  "</interface></node>";

static void method_call(GDBusConnection *connection, const gchar *sender,
  const gchar *object, const gchar *interface, const gchar *method,
  GVariant *parameters, GDBusMethodInvocation *invocation, gpointer data) {
  const gchar *service;
  g_variant_get(parameters, "(&s)", &service);
  printf("%s\t%s\t%s\n", method, sender, service);
  fflush(stdout);
  g_dbus_method_invocation_return_value(invocation, NULL);
  if (g_str_equal(method, "RegisterStatusNotifierItem"))
    g_dbus_connection_emit_signal(connection, NULL, object, interface,
      "StatusNotifierItemRegistered", g_variant_new("(s)", service), NULL);
}
static GVariant *property(GDBusConnection *c, const gchar *sender,
  const gchar *object, const gchar *interface, const gchar *name,
  GError **error, gpointer data) {
  if (g_str_equal(name, "RegisteredStatusNotifierItems")) return g_variant_new_strv(NULL, 0);
  if (g_str_equal(name, "IsStatusNotifierHostRegistered")) return g_variant_new_boolean(TRUE);
  return g_variant_new_int32(0);
}
static const GDBusInterfaceVTable vtable = {method_call, property, NULL};
static void acquired(GDBusConnection *connection, const gchar *name, gpointer data) {
  GError *error = NULL;
  g_dbus_connection_register_object(connection, "/StatusNotifierWatcher",
    info->interfaces[0], &vtable, NULL, NULL, &error);
  if (error) g_error("%s", error->message);
  puts("WATCHER_READY"); fflush(stdout);
}
int main(void) {
  info = g_dbus_node_info_new_for_xml(xml, NULL);
  loop = g_main_loop_new(NULL, FALSE);
  guint owner = g_bus_own_name(G_BUS_TYPE_SESSION, "org.kde.StatusNotifierWatcher",
    G_BUS_NAME_OWNER_FLAGS_NONE, acquired, NULL, NULL, NULL, NULL);
  g_main_loop_run(loop);
  g_bus_unown_name(owner);
  return 0;
}
