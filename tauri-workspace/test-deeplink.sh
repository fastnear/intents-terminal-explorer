#!/bin/bash
# Test script to trigger NEAR deep links

echo "Testing Ratacat NEAR deep links..."
echo ""
echo "Opening test deep links in 3 seconds..."
sleep 3

# Test different deep link types
echo "1. Opening transaction link..."
open "near://tx/AbCdEf123456789"
sleep 1

echo "2. Opening account link..."
open "near://account/alice.near"
sleep 1

echo "3. Opening block link..."
open "near://block/12345678"
sleep 1

echo "4. Opening ratacat link..."
open "near://ratacat"

echo ""
echo "Done! Check the Ratacat window to see the received deep links."
echo ""
echo "Registered URL scheme: near://"
