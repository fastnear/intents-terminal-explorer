/**
 * NEARxTest - E2E Testing Bridge API
 *
 * This module exposes a test API for E2E tests to interact with the NEARx app
 * without relying on fragile canvas pixel inspection.
 *
 * Only loaded when `e2e` feature is enabled during build.
 *
 * @namespace window.NEARxTest
 */

(function() {
    'use strict';

    // Store last route for navigation assertions
    let lastRoute = null;
    let lastCursorIcon = null;

    // Track deep link events
    const deepLinkHistory = [];

    // Initialize the test bridge
    const NEARxTest = {
        /**
         * Get the last route that was navigated to
         * @returns {string|null} Last route or null
         */
        getLastRoute() {
            return lastRoute;
        },

        /**
         * Get cursor icon state (for hover assertions)
         * @returns {boolean} True if cursor is PointingHand
         */
        cursorIsPointer() {
            return lastCursorIcon === 'pointer' || lastCursorIcon === 'PointingHand';
        },

        /**
         * Copy focused pane content to clipboard
         * Uses the platform abstraction layer
         * @returns {Promise<boolean>} True if copy succeeded
         */
        async copyFocused() {
            try {
                // Simulate 'c' key press to trigger copy
                // This matches real user behavior
                const event = new KeyboardEvent('keydown', {
                    key: 'c',
                    code: 'KeyC',
                    bubbles: true,
                    cancelable: true
                });
                document.dispatchEvent(event);

                // Give it time to process
                await new Promise(resolve => setTimeout(resolve, 100));

                return true;
            } catch (error) {
                console.error('[NEARxTest] Copy failed:', error);
                return false;
            }
        },

        /**
         * Get deep link event history
         * @returns {Array<object>} Array of deep link events
         */
        getDeepLinkHistory() {
            return [...deepLinkHistory];
        },

        /**
         * Clear deep link history
         */
        clearDeepLinkHistory() {
            deepLinkHistory.length = 0;
        },

        /**
         * Wait for a specific route to be set
         * @param {string} expectedRoute - Route pattern to wait for
         * @param {number} timeout - Max wait time in ms (default 5000)
         * @returns {Promise<boolean>} True if route matched within timeout
         */
        async waitForRoute(expectedRoute, timeout = 5000) {
            const startTime = Date.now();
            while (Date.now() - startTime < timeout) {
                if (lastRoute && lastRoute.includes(expectedRoute)) {
                    return true;
                }
                await new Promise(resolve => setTimeout(resolve, 100));
            }
            return false;
        },

        /**
         * Simulate keyboard navigation
         * @param {string} key - Key to press (e.g., 'Tab', 'ArrowDown')
         */
        async pressKey(key) {
            const event = new KeyboardEvent('keydown', {
                key,
                code: key,
                bubbles: true,
                cancelable: true
            });
            document.dispatchEvent(event);
            await new Promise(resolve => setTimeout(resolve, 50));
        },

        /**
         * Get OAuth token from localStorage
         * @returns {string|null} Token or null
         */
        getToken() {
            try {
                return localStorage.getItem('nearx.token');
            } catch {
                return null;
            }
        },

        /**
         * Set OAuth token in localStorage
         * @param {string} token - Token to set
         */
        setToken(token) {
            try {
                localStorage.setItem('nearx.token', token);
            } catch (error) {
                console.error('[NEARxTest] Failed to set token:', error);
            }
        },

        /**
         * Invoke Tauri command (wrapper for test assertions)
         * @param {string} cmd - Command name
         * @param {object} args - Command arguments
         * @returns {Promise<any>} Command result
         */
        async invoke(cmd, args = {}) {
            if (!window.__TAURI__?.invoke) {
                throw new Error('Tauri invoke not available');
            }
            return window.__TAURI__.invoke(cmd, args);
        }
    };

    // Listen for hash changes to track routing
    window.addEventListener('hashchange', () => {
        lastRoute = window.location.hash;
        console.log('[NEARxTest] Route changed:', lastRoute);
    });

    // Track initial route
    lastRoute = window.location.hash || '#/';

    // Listen for deep link events if Tauri is available
    if (window.__TAURI__?.event?.listen) {
        window.__TAURI__.event.listen('nearx://open', (event) => {
            const payload = event.payload;
            console.log('[NEARxTest] Deep link event:', payload);
            deepLinkHistory.push({
                timestamp: Date.now(),
                url: payload
            });

            // Update last route if it's a parseable deep link
            if (typeof payload === 'string' && payload.startsWith('nearx://')) {
                lastRoute = payload;
            }
        });
    }

    // Expose the API globally
    window.NEARxTest = NEARxTest;

    console.log('[NEARxTest] Test bridge initialized');
    console.log('[NEARxTest] Available methods:', Object.keys(NEARxTest));
})();
