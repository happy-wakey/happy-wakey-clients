package happywakey

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"strings"
	"time"

	nextloggers "github.com/ores-otel/ores.otel.log/sdk/go"
)

type Client struct {
	BaseURL, Token string
	HTTP           *http.Client
	Telemetry      *nextloggers.Logger
}

func New(baseURL, token string, telemetry *nextloggers.Logger) (*Client, error) {
	u, err := url.Parse(baseURL)
	if err != nil || u.Scheme != "https" {
		return nil, fmt.Errorf("HTTPS required")
	}
	return &Client{strings.TrimRight(baseURL, "/"), token, &http.Client{Timeout: 10 * time.Second, CheckRedirect: func(_ *http.Request, _ []*http.Request) error { return http.ErrUseLastResponse }}, telemetry}, nil
}
func (c *Client) Health(ctx context.Context) (json.RawMessage, error) {
	return c.call(ctx, "health", "GET", "/healthz", nil, false)
}
func (c *Client) ListAlarms(ctx context.Context) (json.RawMessage, error) {
	return c.call(ctx, "list_alarms", "GET", "/v1/alarms", nil, true)
}
func (c *Client) CreateAlarm(ctx context.Context, request any) (json.RawMessage, error) {
	return c.call(ctx, "create_alarm", "POST", "/v1/alarms", request, true)
}
func (c *Client) TransitionOccurrence(ctx context.Context, id string, request any) (json.RawMessage, error) {
	return c.call(ctx, "transition_occurrence", "POST", "/v1/occurrences/"+url.PathEscape(id)+"/transitions", request, true)
}
func (c *Client) PullChanges(ctx context.Context, cursor string, limit int) (json.RawMessage, error) {
	return c.call(ctx, "pull_changes", "GET", "/v1/sync/pull?cursor="+url.QueryEscape(cursor)+"&limit="+strconv.Itoa(limit), nil, true)
}
func (c *Client) PushChanges(ctx context.Context, changes any) (json.RawMessage, error) {
	return c.call(ctx, "push_changes", "POST", "/v1/sync/push", changes, true)
}
func (c *Client) call(ctx context.Context, operation, method, path string, body any, auth bool) (json.RawMessage, error) {
	var reader io.Reader
	if body != nil {
		encoded, err := json.Marshal(body)
		if err != nil {
			return nil, err
		}
		reader = bytes.NewReader(encoded)
	}
	req, err := http.NewRequestWithContext(ctx, method, c.BaseURL+path, reader)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Accept", "application/json")
	if auth {
		if c.Token == "" {
			return nil, fmt.Errorf("Shared Auth bearer token required")
		}
		req.Header.Set("Authorization", "Bearer "+c.Token)
	}
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	response, err := c.HTTP.Do(req)
	if err != nil {
		c.emit(operation, 0, true)
		return nil, err
	}
	defer response.Body.Close()
	payload, readErr := io.ReadAll(io.LimitReader(response.Body, 2<<20))
	if readErr != nil {
		return nil, readErr
	}
	c.emit(operation, response.StatusCode, response.StatusCode >= 400)
	if response.StatusCode < 200 || response.StatusCode >= 300 {
		return nil, fmt.Errorf("Happy Wakey request failed (%d)", response.StatusCode)
	}
	return payload, nil
}
func (c *Client) emit(operation string, status int, failed bool) {
	if c.Telemetry == nil {
		return
	}
	event := c.Telemetry.Info("happy_wakey.client.request")
	if failed {
		event = c.Telemetry.Error("happy_wakey.client.request")
	}
	_ = event.AddFields(map[string]any{"operation": operation, "status": status}).AddTags("happy-wakey", "client").Send()
}
