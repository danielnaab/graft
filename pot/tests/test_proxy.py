"""pot's proxy (pot/core/proxy.py): the host's loopback is never reached (F1, v2.19.0). A name is resolved once, every
address it has is checked, and the connection goes to a checked address, so a name that resolves to loopback (the
host's own name, localhost., a rebinding name) is refused like a loopback literal."""
import asyncio
import socket
import unittest

import proxy


class LoopbackAddress(unittest.TestCase):
    def test_refused(self):
        for a in ("127.0.0.1", "127.255.0.9", "::1", "0.0.0.0", "0.1.2.3", "::", "::ffff:127.0.0.1", "::ffff:0.0.0.0"):
            self.assertTrue(proxy.host_local(a), a)

    def test_others(self):
        for a in ("192.168.1.52", "10.0.0.1", "93.184.215.14", "2001:db8::1", "::ffff:192.0.2.1", "128.0.0.1"):
            self.assertFalse(proxy.host_local(a), a)


class Resolve(unittest.TestCase):
    def run_(self, coro):
        return asyncio.run(coro)

    def test_loopback_literal(self):
        with self.assertRaises(proxy.Loopback) as e:
            self.run_(proxy.resolve("127.0.0.1", 1))
        self.assertEqual(e.exception.address, "127.0.0.1")

    def test_name_that_resolves_to_loopback(self):
        with self.assertRaises(proxy.Loopback):
            self.run_(proxy.resolve("localhost", 1))

    def test_own_hostname(self):
        # on a host whose name resolves to loopback (as platform-vm's does), the name is refused too
        name = socket.gethostname()
        try:
            addrs = {i[4][0] for i in socket.getaddrinfo(name, 1, type=socket.SOCK_STREAM)}
        except OSError:
            self.skipTest("the host's name does not resolve here")
        if not any(proxy.host_local(a) for a in addrs):
            self.skipTest("the host's name does not resolve to loopback here")
        with self.assertRaises(proxy.Loopback):
            self.run_(proxy.resolve(name, 1))

    def test_any_loopback_address_refuses(self):
        # a name with a public and a loopback address is refused, whichever comes first
        async def fake(host, port, **kw):
            return [(socket.AF_INET, socket.SOCK_STREAM, 6, "", ("192.0.2.1", port)),
                    (socket.AF_INET, socket.SOCK_STREAM, 6, "", ("127.0.0.1", port))]
        with self.assertRaises(proxy.Loopback):
            self.run_(proxy.resolve("mixed.example", 1, getaddrinfo=fake))

    def test_checked_addresses(self):
        async def fake(host, port, **kw):
            return [(socket.AF_INET, socket.SOCK_STREAM, 6, "", ("192.0.2.1", port)),
                    (socket.AF_INET6, socket.SOCK_STREAM, 6, "", ("2001:db8::1", port, 0, 0)),
                    (socket.AF_INET, socket.SOCK_STREAM, 6, "", ("192.0.2.1", port))]
        self.assertEqual(self.run_(proxy.resolve("ok.example", 443, getaddrinfo=fake)), ["192.0.2.1", "2001:db8::1"])


class Denial(unittest.TestCase):
    def test_text(self):
        self.assertEqual(proxy.loopback_text("work", "platform-vm:18731", "127.0.0.2"),
                         "pot (work): the run tried platform-vm:18731, the host's loopback (127.0.0.2): "
                         "a run cannot reach it (host-loopback is reserved for a later release).\n")


    def test_same_as_the_exit_line(self):
        import compose
        self.assertEqual(proxy.loopback_text("work", "h:1", "::1"), compose.loopback_text("work", "h:1", "::1") + "\n")
        self.assertEqual(compose.loopback_text("dispatch", "h:1", "::1", 3),
                         "pot (dispatch): the run tried h:1, the host's loopback (::1, 3x): a run cannot reach it "
                         "(host-loopback is reserved for a later release).")


if __name__ == "__main__":
    unittest.main()
